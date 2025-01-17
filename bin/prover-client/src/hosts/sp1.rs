use std::sync::LazyLock;

use strata_primitives::proof::ProofContext;
use strata_sp1_adapter::SP1Host;
use strata_sp1_guest_builder::*;

pub static BTC_BLOCKSPACE_HOST: LazyLock<SP1Host> = std::sync::LazyLock::new(|| {
    {
        SP1Host::new_from_bytes(
            &GUEST_BTC_BLOCKSPACE_ELF,
            &GUEST_BTC_BLOCKSPACE_PK,
            &GUEST_BTC_BLOCKSPACE_VK,
        )
    }
});

pub static L1_BATCH_HOST: LazyLock<SP1Host> = std::sync::LazyLock::new(|| {
    {
        SP1Host::new_from_bytes(&GUEST_L1_BATCH_ELF, &GUEST_L1_BATCH_PK, &GUEST_L1_BATCH_VK)
    }
});

pub static EVM_EE_STF_HOST: LazyLock<SP1Host> = std::sync::LazyLock::new(|| {
    {
        SP1Host::new_from_bytes(
            &GUEST_EVM_EE_STF_ELF,
            &GUEST_EVM_EE_STF_PK,
            &GUEST_EVM_EE_STF_VK,
        )
    }
});

pub static CL_STF_HOST: LazyLock<SP1Host> = std::sync::LazyLock::new(|| {
    {
        SP1Host::new_from_bytes(&GUEST_CL_STF_ELF, &GUEST_CL_STF_PK, &GUEST_CL_STF_VK)
    }
});

pub static CL_AGG_HOST: LazyLock<SP1Host> = std::sync::LazyLock::new(|| {
    {
        SP1Host::new_from_bytes(&GUEST_CL_AGG_ELF, &GUEST_CL_AGG_PK, &GUEST_CL_AGG_VK)
    }
});

pub static CHECKPOINT_HOST: LazyLock<SP1Host> = std::sync::LazyLock::new(|| {
    {
        SP1Host::new_from_bytes(
            &GUEST_CHECKPOINT_ELF,
            &GUEST_CHECKPOINT_PK,
            &GUEST_CHECKPOINT_VK,
        )
    }
});

/// Returns a reference to the appropriate `SP1Host` instance based on the given [`ProofContext`].
///
/// This function maps the `ProofContext` variant to its corresponding static [`SP1Host`]
/// instance, allowing for efficient host selection for different proof types.
pub fn get_host(id: &ProofContext) -> &'static SP1Host {
    match id {
        ProofContext::BtcBlockspace(..) => &BTC_BLOCKSPACE_HOST,
        ProofContext::L1Batch(..) => &L1_BATCH_HOST,
        ProofContext::EvmEeStf(..) => &EVM_EE_STF_HOST,
        ProofContext::ClStf(..) => &CL_STF_HOST,
        ProofContext::ClAgg(..) => &CL_AGG_HOST,
        ProofContext::Checkpoint(..) => &CHECKPOINT_HOST,
    }
}
