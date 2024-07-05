<<<<<<< HEAD
=======
use std::{
    fmt::{self, Debug},
    str
};

use alpen_vertex_primitives::l1::L1Tx;
use alpen_vertex_primitives::prelude::*;
>>>>>>> 6ffc2bd (feat: clientStatus RPC)
use arbitrary::Arbitrary;
use borsh::{BorshDeserialize, BorshSerialize};

use alpen_vertex_primitives::prelude::*;

use crate::{block_template, exec_update, id::L2BlockId, l1};

impl fmt::Display for L2BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0; 64];
        hex::encode_to_slice(self.0.0, &mut buf).expect("buf: enc hex");
        f.write_str(unsafe { str::from_utf8_unchecked(&buf) })
    }
}

/// Full contents of the bare L2 block.
#[derive(Clone, Debug, Eq, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct L2Block {
    /// Header that links the block into the L2 block chain and carries the
    /// block's credential from a sequencer.
    header: L2BlockHeader,

    /// Body that contains the bulk of the data.
    body: L2BlockBody,
}

impl L2Block {
    pub fn new(header: L2BlockHeader, body: L2BlockBody) -> Self {
        Self { header, body }
    }

    pub fn header(&self) -> &L2BlockHeader {
        &self.header
    }

    pub fn l1_segment(&self) -> &L1Segment {
        &self.body.l1_segment
    }

    pub fn exec_segment(&self) -> &ExecSegment {
        &self.body.exec_segment
    }
}

/// Careful impl that makes the header consistent with the body.  But the prev
/// block is always 0 and the state root is random.
impl<'a> Arbitrary<'a> for L2Block {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let body = L2BlockBody::arbitrary(u)?;
        let idx = u64::arbitrary(u)?;
        let ts = u64::arbitrary(u)?;
        let prev = L2BlockId::from(Buf32::zero());
        let sr = Buf32::arbitrary(u)?;
        let tmplt = block_template::create_header_template(idx, ts, prev, &body, sr);
        let header = tmplt.complete_with(Buf64::arbitrary(u)?);
        Ok(Self::new(header, body))
    }
}

/// Block header that forms the chain we use to reach consensus.
#[derive(Clone, Debug, Eq, PartialEq, Arbitrary, BorshDeserialize, BorshSerialize)]
pub struct L2BlockHeader {
    /// Block index, obviously.
    pub(crate) block_idx: u64,

    /// Timestamp the block was (intended to be) published at.
    pub(crate) timestamp: u64,

    /// Hash of the previous block, to form the blockchain.
    pub(crate) prev_block: L2BlockId,

    /// Hash of the L1 segment.
    pub(crate) l1_segment_hash: Buf32,

    /// Hash of the exec segment.
    // TODO ideally this is just the EL header hash, not the hash of the full payload
    pub(crate) exec_segment_hash: Buf32,

    /// State root that commits to the overall state of the rollup, commits to
    /// both the CL state and EL state.
    // TODO figure out the structure of this
    pub(crate) state_root: Buf32,

    /// Signature from this block's proposer.
    pub(crate) signature: Buf64,
}

impl L2BlockHeader {
    pub fn blockidx(&self) -> u64 {
        self.block_idx
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn parent(&self) -> &L2BlockId {
        &self.prev_block
    }

    pub fn l1_payload_hash(&self) -> &Buf32 {
        &self.l1_segment_hash
    }

    pub fn exec_payload_hash(&self) -> &Buf32 {
        &self.exec_segment_hash
    }

    pub fn state_root(&self) -> &Buf32 {
        &self.state_root
    }

    pub fn sig(&self) -> &Buf64 {
        &self.signature
    }

    /// Computes the blockid with SHA256.
    // TODO should this be poseidon?
    pub fn get_blockid(&self) -> L2BlockId {
        let buf = borsh::to_vec(self).expect("block: compute blkid");
        let h = <sha2::Sha256 as digest::Digest>::digest(&buf);
        L2BlockId::from(Buf32::from(<[u8; 32]>::from(h)))
    }
}

// TODO: add this behind a feature flag so that this can only be used for the testing purposes
impl L2BlockHeader {
    pub fn set_parent_and_idx(&mut self, parent: L2BlockId, idx: u64) {
        self.prev_block = parent;
        self.block_idx = idx;
    }
}

/// Contains the additional payloads within the L2 block.
<<<<<<< HEAD
#[derive(Clone, Debug, Eq, PartialEq, Arbitrary, BorshSerialize, BorshDeserialize)]
=======
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, Arbitrary)]
>>>>>>> 6ffc2bd (feat: clientStatus RPC)
pub struct L2BlockBody {
    l1_segment: L1Segment,
    exec_segment: ExecSegment,
}

impl L2BlockBody {
    pub fn new(l1_segment: L1Segment, exec_segment: ExecSegment) -> Self {
        Self {
            l1_segment,
            exec_segment,
        }
    }

    pub fn l1_segment(&self) -> &L1Segment {
        &self.l1_segment
    }

    pub fn exec_segment(&self) -> &ExecSegment {
        &self.exec_segment
    }
}

/// Container for additional messages that we've observed from the L1, if there
/// are any.
<<<<<<< HEAD
#[derive(Clone, Debug, Eq, PartialEq, Arbitrary, BorshSerialize, BorshDeserialize)]
=======
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, Arbitrary)]
>>>>>>> 6ffc2bd (feat: clientStatus RPC)
pub struct L1Segment {
    /// New headers that we've seen from L1 that we didn't see in the previous
    /// L2 block.
    new_payloads: Vec<l1::L1HeaderPayload>,
}

impl L1Segment {
    pub fn new(new_payloads: Vec<l1::L1HeaderPayload>) -> Self {
        Self { new_payloads }
    }

    pub fn new_empty() -> Self {
        Self::new(Vec::new())
    }
}

/// Information relating to how to update the execution layer.
///
/// Right now this just contains a single execution update since we only have a
/// single execution environment in our execution layer.
#[derive(Clone, Debug, Eq, PartialEq, Arbitrary, BorshSerialize, BorshDeserialize)]
pub struct ExecSegment {
    /// Update payload for the single execution environment.
    update: exec_update::ExecUpdate,
}

impl ExecSegment {
    pub fn new(update: exec_update::ExecUpdate) -> Self {
        Self { update }
    }

    /// The EE update payload.
    ///
    /// This might be replaced with a totally different scheme if we have
    /// multiple EEs.
    pub fn update(&self) -> &exec_update::ExecUpdate {
        &self.update
    }
}
