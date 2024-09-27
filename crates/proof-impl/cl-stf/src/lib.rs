//! This crate implements the proof of the chain state transition function (STF) for L2 blocks,
//! verifying the correct state transitions as new L2 blocks are processed.

use alpen_express_primitives::params::Params;
use alpen_express_state::block_validation::{check_block_credential, validate_block_segments};
pub use alpen_express_state::{block::L2Block, chain_state::ChainState, state_op::StateCache};

/// Verifies an L2 block and applies the chains state transition if the block is valid.
pub fn verify_and_transition(
    prev_chstate: ChainState,
    new_l2_block: L2Block,
    chain_params: Params,
) -> ChainState {
    verify_l2_block(&new_l2_block, &chain_params);
    apply_state_transition(prev_chstate, &new_l2_block, &chain_params)
}

/// Verifies the L2 block.
fn verify_l2_block(block: &L2Block, chain_params: &Params) {
    // Assert that the block has been signed by the designated signer
    assert!(
        check_block_credential(block.header(), chain_params),
        "Block credential verification failed"
    );

    // Assert that the block body and header are consistent
    assert!(
        validate_block_segments(block),
        "Block credential verification failed"
    )
}

/// Applies a state transition for a given L2 block.
fn apply_state_transition(
    prev_chstate: ChainState,
    new_l2_block: &L2Block,
    chain_params: &Params,
) -> ChainState {
    let mut state_cache = StateCache::new(prev_chstate);

    express_chaintsn::transition::process_block(
        &mut state_cache,
        new_l2_block.header(),
        new_l2_block.body(),
        chain_params.rollup(),
    )
    .expect("Failed to process the L2 block");

    state_cache.state().to_owned()
}
