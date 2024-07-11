use std::fmt;

use arbitrary::Arbitrary;
use borsh::{BorshDeserialize, BorshSerialize};

use alpen_vertex_primitives::buf::Buf32;

/// ID of an L2 block, usually the hash of its root header.
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary, BorshSerialize, BorshDeserialize,
)]
pub struct L2BlockId(Buf32);

impl From<Buf32> for L2BlockId {
    fn from(value: Buf32) -> Self {
        Self(value)
    }
}

impl From<L2BlockId> for Buf32 {
    fn from(value: L2BlockId) -> Self {
        value.0
    }
}

impl fmt::Debug for L2BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for L2BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}