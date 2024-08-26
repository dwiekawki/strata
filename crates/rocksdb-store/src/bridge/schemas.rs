use alpen_express_db::entities::bridge_tx_state::BridgeTxState;
use alpen_express_primitives::buf::Buf32;

use crate::{
    define_table_with_default_codec, define_table_with_seek_key_codec, define_table_without_codec,
    impl_borsh_value_codec,
};

define_table_with_seek_key_codec!(
    /// A table to store mapping of [`Txid`] to [`CollectedSigs`].
    (BridgeSigTxidSchema) u64 => Buf32
);

define_table_with_default_codec!(
    /// A table to store [`CollectedSigs`]
    (BridgeSigSchema) Buf32 => BridgeTxState
);
