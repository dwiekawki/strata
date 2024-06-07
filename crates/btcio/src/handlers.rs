use std::sync::Arc;

use alpen_vertex_db::traits::{L1DataProvider, L1DataStore};
use alpen_vertex_primitives::buf::Buf32;
use alpen_vertex_primitives::{l1::L1BlockManifest, utils::generate_l1_tx};

use bitcoin::consensus::serialize;
use bitcoin::hashes::Hash;
use tokio::sync::mpsc;
use tracing::warn;

use crate::reader::{BlockData, L1Data};

fn block_data_to_manifest(blockdata: BlockData) -> L1BlockManifest {
    let blockid = Buf32(
        blockdata
            .block()
            .block_hash()
            .to_raw_hash()
            .to_byte_array()
            .into(),
    );
    let root = blockdata
        .block()
        .witness_root()
        .map(|x| x.to_byte_array())
        .unwrap_or_default();
    let header = serialize(&blockdata.block().header);

    L1BlockManifest::new(blockid, header, Buf32(root.into()))
}

pub async fn bitcoin_data_handler<D>(
    l1db: Arc<D>,
    mut receiver: mpsc::Receiver<L1Data>,
) -> anyhow::Result<()>
where
    D: L1DataProvider + L1DataStore,
{
    loop {
        if let Some(data) = receiver.recv().await {
            match data {
                L1Data::BlockData(blockdata) => {
                    let manifest = block_data_to_manifest(blockdata.clone());
                    let l1txs: Vec<_> = blockdata
                        .relevant_txn_indices()
                        .iter()
                        .enumerate()
                        .map(|(i, _)| generate_l1_tx(i as u32, blockdata.block()))
                        .collect();
                    l1db.put_block_data(blockdata.block_num(), manifest, l1txs)?;
                }

                L1Data::Reorg(revert_to) => {}
            }
        } else {
            warn!("Bitcoin reader sent None blockdata");
        }
    }
}
