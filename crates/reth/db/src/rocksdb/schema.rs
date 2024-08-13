use reth_primitives::B256;

use alpen_express_rocksdb::define_table_with_seek_key_codec;
use alpen_express_rocksdb::define_table_without_codec;
use alpen_express_rocksdb::impl_borsh_value_codec;

// NOTE: using seek_key_codec as B256 does not derive borsh serialization
define_table_with_seek_key_codec!(
    /// store of block witness data. Data stored as serialized bytes for directly serving in rpc
    (BlockWitnessSchema) B256 => Vec<u8>
);