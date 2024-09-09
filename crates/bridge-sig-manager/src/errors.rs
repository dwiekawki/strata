//! Defines errors associated with the signature manager.

use bitcoin::{
    psbt::{self, ExtractTxError},
    sighash::TaprootError,
};
use thiserror::Error;

/// Errors that may occur during the signing and aggregation of signatures for a particular
/// [`Psbt`](bitcoin::Psbt).
#[derive(Debug, Clone, Error)]
pub enum BridgeSigError {
    /// Failed to build a [`Pbst`] from the unsigned transaction. This can happen if the
    /// transaction that is being converted to a psbt contains a non-empty script sig or
    /// witness fields.
    #[error("failed to build psbt: {0}")]
    BuildPsbtFailed(String),

    /// No input exists for the given index in the psbt.
    #[error("no input exists for the given index in the PSBT")]
    InputIndexOutOfBounds,

    /// The provided signature is not valid for the given transaction and pubkey.
    #[error("invalid signature")]
    InvalidSignature,

    /// The pubkey is not part of the signatories required for the psbt.
    #[error("pubkey is not a required signatory")]
    UnauthorizedPubkey,

    /// Error occurred while persisting/accessing signatures.
    #[error("error persisting/accessing signatures")]
    StorageError,

    /// Transaction for the provided signature does not exist in state/storage.
    #[error("transaction does not exist")]
    TransactionNotFound,

    /// The transaction is not fully signed yet.
    #[error("transaction not fully signed yet")]
    NotFullySigned,

    /// The witness stack in the transaction does not contain the script and control block.
    #[error("initial witness block cannot be empty")]
    EmptyWitnessBlock,

    /// Failed to create signed transaction after all signatures have been collected.
    #[error("failed to build signed transaction due to {0}")]
    BuildSignedTxFailed(#[from] ExtractTxError),

    /// Failed to produce taproot sig hash
    #[error("failed to create taproot sig hash due to {0}")]
    SighashError(#[from] TaprootError),
}

/// Manual implementation of conversion for [`psbt::Error`] <-> [`BridgeSigError`] as the former
/// does not implement [`Clone`] ¯\_(ツ)_/¯.
impl From<psbt::Error> for BridgeSigError {
    fn from(value: psbt::Error) -> Self {
        Self::BuildPsbtFailed(value.to_string())
    }
}

/// Result type alias for the signature manager with [`BridgeSigError`] as the Error variant.
pub type BridgeSigResult<T> = Result<T, BridgeSigError>;