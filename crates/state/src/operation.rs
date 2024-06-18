//! Operations that a state transition emits to update the new state and control
//! the client's high level state.

use std::sync::Arc;

use arbitrary::Arbitrary;
use borsh::{BorshDeserialize, BorshSerialize};

use crate::block::L2BlockId;
use crate::consensus::{ConsensusChainState, ConsensusState};
use crate::l1::L1BlockId;

/// Output of a consensus state transition.  Both the consensus state writes and
/// sync actions.
#[derive(Clone, Debug, Eq, PartialEq, Arbitrary, BorshDeserialize, BorshSerialize)]
pub struct ConsensusOutput {
    writes: Vec<ConsensusWrite>,
    actions: Vec<SyncAction>,
}

impl ConsensusOutput {
    pub fn new(writes: Vec<ConsensusWrite>, actions: Vec<SyncAction>) -> Self {
        Self { writes, actions }
    }

    pub fn writes(&self) -> &[ConsensusWrite] {
        &self.writes
    }

    pub fn actions(&self) -> &[SyncAction] {
        &self.actions
    }

    pub fn into_parts(self) -> (Vec<ConsensusWrite>, Vec<SyncAction>) {
        (self.writes, self.actions)
    }
}

/// Describes possible writes to chain state that we can make.  We use this
/// instead of directly modifying the chain state to reduce the volume of data
/// that we have to clone and save to disk with each sync event.
#[derive(Clone, Debug, Eq, PartialEq, Arbitrary, BorshDeserialize, BorshSerialize)]
pub enum ConsensusWrite {
    /// Completely replace the full state with a new instance.
    Replace(Box<ConsensusState>),

    /// Replace just the L2 blockchain consensus-layer state with a new
    /// instance.
    ReplaceChainState(Box<ConsensusChainState>),

    /// Queue an L2 block to be accepted.
    AcceptL2Block(L2BlockId),

    /// Rolls back L1 blocks to this block ID.
    RollbackL1BlocksTo(L1BlockId),

    /// Insert L1 blocks into the pending queue.
    AcceptL1Block(L1BlockId),

    /// Updates the buried block index to a higher index.
    UpdateBuried(u64),
}

/// Actions the consensus state machine directs the node to take to update its
/// own bookkeeping.  These should not be able to fail.
#[derive(Clone, Debug, Eq, PartialEq, Arbitrary, BorshDeserialize, BorshSerialize)]
pub enum SyncAction {
    /// Extends our externally-facing tip to a new block ID.  This might trigger
    /// a reorg of some unfinalized blocks.  We probably won't roll this block
    /// back but we haven't seen it proven on-chain yet.  This is also where
    /// we'd build a new block if it's our turn to.
    UpdateTip(L2BlockId),

    /// Marks an L2 blockid as invalid and we won't follow any chain that has
    /// it, and will reject it from our peers.
    // TODO possibly we should have some way of marking a block invalid through
    // preliminary checks before writing a sync event we then have to check,
    // this should be investigated more
    MarkInvalid(L2BlockId),

    /// Finalizes a block, indicating that it won't be reverted.
    FinalizeBlock(L2BlockId),
}

/// Applies consensus writes to the provided consensus state.
pub fn apply_writes_to_state(
    state: &mut ConsensusState,
    writes: impl Iterator<Item = ConsensusWrite>,
) {
    for w in writes {
        use ConsensusWrite::*;
        match w {
            Replace(cs) => *state = *cs,
            ReplaceChainState(ccs) => state.chain_state = *ccs,
            RollbackL1BlocksTo(l1blkid) => {
                let pos = state.recent_l1_blocks.iter().position(|b| *b == l1blkid);
                let Some(pos) = pos else {
                    // TODO better logging, maybe make this an actual error
                    panic!("operation: emitted invalid write");
                };
                state.recent_l1_blocks.truncate(pos);
            }
            AcceptL1Block(l1blkid) => state.recent_l1_blocks.push(l1blkid),
            AcceptL2Block(blkid) => state.pending_l2_blocks.push_back(blkid),
            UpdateBuried(new_idx) => {
                // Check that it's increasing.
                let old_idx = state.buried_l1_height;
                if old_idx >= new_idx {
                    panic!("operation: emitted non-greater buried height");
                }

                // Check that it's not higher than what we know about.
                let diff = (new_idx - old_idx) as usize;
                if diff > state.recent_l1_blocks.len() {
                    panic!("operation: new buried height above known L1 tip");
                }

                // If everything checks out we can just remove them.
                let blocks = state.recent_l1_blocks.drain(..diff).collect::<Vec<_>>();
                state.buried_l1_height = new_idx;

                // TODO merge these blocks into the L1 MMR in the chain state if
                // we haven't already
            }
        }
    }
}
