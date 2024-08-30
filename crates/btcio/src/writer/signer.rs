use std::sync::Arc;

use alpen_express_db::types::{BlobEntry, L1TxEntry};
use alpen_express_primitives::buf::Buf32;
use bitcoin::Transaction;
use tracing::debug;

use super::{builder::build_inscription_txs, config::WriterConfig};
use crate::{
    broadcaster::L1BroadcastHandle,
    rpc::traits::{L1Client, SeqL1Client},
};

type BlobIdx = u64;

/// Create inscription transactions corresponding to a [`BlobEntry`].
///
/// This is useful when receiving a new intent as well as when
/// broadcasting fails because the input UTXOs have been spent
/// by something else already.
pub async fn create_and_sign_blob_inscriptions(
    blobentry: &BlobEntry,
    bhandle: &L1BroadcastHandle,
    client: Arc<impl L1Client + SeqL1Client>,
    config: &WriterConfig,
) -> anyhow::Result<(Buf32, Buf32)> {
    let (commit, reveal) = build_inscription_txs(&blobentry.blob, &client, config).await?;

    debug!("Signing commit transaction {}", commit.compute_txid());
    let signed_commit: Transaction = client.sign_raw_transaction_with_wallet(commit).await?;

    let cid: Buf32 = signed_commit.compute_txid().into();
    let rid: Buf32 = reveal.compute_txid().into();

    let centry = L1TxEntry::from_tx(&signed_commit);
    let rentry = L1TxEntry::from_tx(&reveal);

    // These don't need to be atomic. It will be handled by writer task if it does not find both
    // commit-reveal txs in db by triggering re-signing.
    let _ = bhandle.insert_new_tx_entry(cid, centry).await?;
    let _ = bhandle.insert_new_tx_entry(rid, rentry).await?;
    Ok((cid, rid))
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use alpen_express_db::types::{BlobEntry, BlobL1Status};
    use alpen_express_primitives::hash;

    use super::*;
    use crate::{
        test_utils::TestBitcoinClient,
        writer::test_utils::{get_broadcast_handle, get_config, get_inscription_ops},
    };

    #[tokio::test]
    async fn test_create_and_sign_blob_inscriptions() {
        let iops = get_inscription_ops();
        let bcast_handle = get_broadcast_handle();
        let client = Arc::new(TestBitcoinClient::new(1));
        let config = get_config();

        // First insert an unsigned blob
        let entry = BlobEntry::new_unsigned([1; 100].to_vec());

        assert_eq!(entry.status, BlobL1Status::Unsigned);
        assert_eq!(entry.commit_txid, Buf32::zero());
        assert_eq!(entry.reveal_txid, Buf32::zero());

        let intent_hash = hash::raw(&entry.blob);
        iops.put_blob_entry_async(intent_hash, entry.clone())
            .await
            .unwrap();

        let (cid, rid) =
            create_and_sign_blob_inscriptions(&entry, bcast_handle.as_ref(), client, &config)
                .await
                .unwrap();

        // Check if corresponding txs exist in db
        let ctx = bcast_handle.get_tx_entry_by_id_async(cid).await.unwrap();
        let rtx = bcast_handle.get_tx_entry_by_id_async(rid).await.unwrap();
        assert!(ctx.is_some());
        assert!(rtx.is_some());
    }
}
