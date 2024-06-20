//! Executes duties.

use std::collections::HashMap;
use std::sync::Arc;
use std::{thread, time};

use borsh::{BorshDeserialize, BorshSerialize};
use tokio::sync::broadcast;
use tracing::*;

use alpen_vertex_db::traits::{ConsensusStateProvider, Database, L2DataProvider, L2DataStore};
use alpen_vertex_evmctl::engine::{ExecEngineCtl, PayloadStatus};
use alpen_vertex_evmctl::errors::EngineError;
use alpen_vertex_evmctl::messages::{ExecPayloadData, PayloadEnv};
use alpen_vertex_primitives::buf::{Buf32, Buf64};
use alpen_vertex_state::block::{ExecSegment, L1Segment, L2Block, L2BlockBody};
use alpen_vertex_state::block_template;
use alpen_vertex_state::consensus::ConsensusState;

use crate::duties::{self, Duty, DutyBatch, Identity};
use crate::duty_extractor;
use crate::errors::Error;
use crate::message::ConsensusUpdateNotif;

#[derive(Clone, Debug, BorshDeserialize, BorshSerialize)]
pub enum IdentityKey {
    Sequencer(Buf32),
}

#[derive(Clone, Debug)]
pub struct IdentityData {
    ident: Identity,
    key: IdentityKey,
}

pub fn duty_tracker_task<D: Database, E: ExecEngineCtl>(
    mut state: broadcast::Receiver<ConsensusUpdateNotif>,
    batch_queue: broadcast::Sender<DutyBatch>,
    ident: Identity,
    database: Arc<D>,
) {
    let mut duties_tracker = duties::DutyTracker::new_empty();

    loop {
        let update = match state.blocking_recv() {
            Ok(u) => u,
            Err(broadcast::error::RecvError::Closed) => {
                break;
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                // TODO maybe check the things we missed, but this is fine for now
                warn!(%skipped, "overloaded, skipping indexing some duties");
                continue;
            }
        };

        let ev_idx = update.sync_event_idx();
        trace!(%ev_idx, "new consensus state, updating duties");

        if let Err(e) = update_tracker(
            &mut duties_tracker,
            update.new_state(),
            &ident,
            database.as_ref(),
        ) {
            error!(err = %e, "failed to update duties tracker");
        }

        // Publish the new batch.
        let batch = DutyBatch::new(ev_idx, duties_tracker.duties().to_vec());
        if !batch_queue.send(batch).is_ok() {
            warn!("failed to publish new duties batch");
        }
    }

    info!("duty extractor task exiting");
}

fn update_tracker<D: Database>(
    tracker: &mut duties::DutyTracker,
    state: &ConsensusState,
    ident: &Identity,
    database: &D,
) -> Result<(), Error> {
    let new_duties = duty_extractor::extract_duties(state, &ident, database)?;
    // TODO update the tracker with the new duties and state data

    // Figure out the block slot from the tip blockid.
    // TODO include the block slot in the consensus state
    let tip_blkid = state.chain_state().chain_tip_blockid();
    let l2prov = database.l2_provider();
    let block = l2prov
        .get_block_data(tip_blkid)?
        .ok_or(Error::MissingL2Block(tip_blkid))?;
    let block_idx = block.header().blockidx();
    let ts = time::Instant::now(); // FIXME XXX use .timestamp()!!!

    // TODO figure out which blocks were finalized
    let newly_finalized = Vec::new();
    let tracker_update = duties::StateUpdate::new(block_idx, ts, newly_finalized);
    tracker.update(&tracker_update);

    // Now actually insert the new duties.
    tracker.add_duties(tip_blkid, block_idx, new_duties.into_iter());

    Ok(())
}

pub fn duty_dispatch_task<
    D: Database + Sync + Send + 'static,
    E: ExecEngineCtl + Sync + Send + 'static,
>(
    mut updates: broadcast::Receiver<DutyBatch>,
    ident: IdentityData,
    database: Arc<D>,
    engine: Arc<E>,
) {
    let mut pending_duties: HashMap<u64, thread::JoinHandle<()>> = HashMap::new();

    // TODO still need some stuff here to decide if we're fully synced and
    // *should* dispatch duties

    loop {
        let update = match updates.blocking_recv() {
            Ok(u) => u,
            Err(broadcast::error::RecvError::Closed) => {
                break;
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                warn!(%skipped, "overloaded, skipping dispatching some duties");
                continue;
            }
        };

        // TODO check pending_duties to remove any completed duties

        for duty in update.duties() {
            let id = duty.id();

            // Skip any duties we've already dispatched.
            if pending_duties.contains_key(&id) {
                continue;
            }

            // Clone some things, spawn the task, then remember the join handle.
            // TODO make this use a thread pool
            let d = duty.duty().clone();
            let ik = ident.key.clone();
            let db = database.clone();
            let e = engine.clone();
            let join = thread::spawn(move || duty_exec_task(d, ik, db, e));
            pending_duties.insert(id, join);
        }
    }

    info!("duty dispatcher task exiting");
}

fn duty_exec_task<D: Database, E: ExecEngineCtl>(
    duty: Duty,
    ik: IdentityKey,
    database: Arc<D>,
    engine: Arc<E>,
) {
    if let Err(e) = perform_duty(&duty, &ik, database.as_ref(), engine.as_ref()) {
        error!(err = %e, "error performing duty");
    } else {
        debug!("completed duty successfully");
    }
}

fn perform_duty<D: Database, E: ExecEngineCtl>(
    duty: &Duty,
    ik: &IdentityKey,
    database: &D,
    engine: &E,
) -> Result<(), Error> {
    match duty {
        Duty::SignBlock(data) => {
            let slot = data.slot();
            sign_block(slot, ik, database, engine)?;
            Ok(())
        }
    }
}

fn sign_block<D: Database, E: ExecEngineCtl>(
    slot: u64,
    ik: &IdentityKey,
    database: &D,
    engine: &E,
) -> Result<(), Error> {
    debug!(%slot, "prepating to publish block");

    // Check the block we were supposed to build isn't already in the database,
    // if so then just republish that.  This checks that there just if we have a
    // block at that height, which for now is the same thing.
    let l2prov = database.l2_provider();
    let blocks_at_slot = l2prov.get_blocks_at_height(slot)?;
    if !blocks_at_slot.is_empty() {
        warn!(%slot, "was turn to propose block, but found block in database already");
        return Ok(());
    }

    // TODO get the consensus state this duty was created in response to and
    // pull out the current tip block from it
    // XXX this is really bad as-is
    let cs_prov = database.consensus_state_provider();
    let ckpt_idx = cs_prov.get_last_checkpoint_idx()?; // FIXME this isn't what this is for
    let last_cstate = cs_prov
        .get_state_checkpoint(ckpt_idx)?
        .expect("dutyexec: get state checkpoint");
    let prev_block = last_cstate.chain_state().chain_tip_blockid();

    // Start preparing the EL payload.
    let ts = now_millis();
    let prev_global_sr = Buf32::zero(); // TODO
    let safe_l1_block = Buf32::zero(); // TODO
    let payload_env = PayloadEnv::new(ts, prev_global_sr, safe_l1_block, Vec::new());
    let key = engine.prepare_payload(payload_env)?;
    trace!(%slot, "submitted EL payload job, waiting for completion");

    // TODO Pull data from CSM state that we've observed from L1, including new
    // headers or any headers needed to perform a reorg if necessary.
    let l1_seg = L1Segment::new(Vec::new(), Vec::new());

    // Wait 2 seconds for the block to be finished.
    // TODO Pull data from state about the new safe L1 hash, prev state roots,
    // etc. to assemble the payload env for this block.
    let wait = time::Duration::from_millis(100);
    let timeout = time::Duration::from_millis(3000);
    let Some(payload_data) = poll_status_loop(key, engine, wait, timeout)? else {
        // TODO better error message
        return Err(Error::Other("EL block assembly timed out".to_owned()));
    };
    trace!(%slot, "finished EL payload job");

    // TODO improve how we assemble the exec segment, since this is bodging out
    // the inputs/outputs should be structured
    let exec_seg = ExecSegment::new(payload_data.el_payload().to_owned());

    // Assemble the body and the header template.
    let body = L2BlockBody::new(l1_seg, exec_seg);
    let state_root = Buf32::zero(); // TODO compute this from the different parts
    let tmplt = block_template::create_header_template(slot, ts, prev_block, &body, state_root);
    let header_sig = Buf64::zero(); // TODO actually sign it
    let final_header = tmplt.complete_with(header_sig);
    let blkid = final_header.get_blockid();
    let final_block = L2Block::new(final_header, body);
    info!(%slot, ?blkid, "finished building new block");

    // Store the block in the database.
    let l2store = database.l2_store();
    l2store.put_block_data(final_block.clone())?;
    debug!(?blkid, "wrote block to datastore");

    // TODO push the block into the CSM and publish it for all to see

    Ok(())
}

/// Returns the current unix time as milliseconds.
// TODO maybe we should use a time source that is possibly more consistent with
// the rest of the network for this?
fn now_millis() -> u64 {
    time::UNIX_EPOCH.elapsed().unwrap().as_millis() as u64
}

fn poll_status_loop<E: ExecEngineCtl>(
    job: u64,
    engine: &E,
    wait: time::Duration,
    timeout: time::Duration,
) -> Result<Option<ExecPayloadData>, EngineError> {
    let start = time::Instant::now();
    loop {
        // Sleep at the beginning since the first iter isn't likely to have it
        // ready.
        thread::sleep(wait);

        // Check the payload for the result.
        let payload = engine.get_payload_status(job)?;
        if let PayloadStatus::Ready(pl) = payload {
            return Ok(Some(pl));
        }

        // If we've waited too long now.
        if time::Instant::now() - start > timeout {
            warn!(%job, "payload build job timed out");
            break;
        }
    }

    Ok(None)
}
