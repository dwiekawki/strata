// NOTE: SP1 prover runs in release mode only; therefore run the tests on release mode only
#[cfg(feature = "prover")]
mod test {
    use express_proofimpl_evm_ee_stf::{ELProofInput, ELProofPublicParams};
    use express_sp1_adapter::{SP1Host, SP1Verifier};
    use express_sp1_guest_builder::GUEST_EVM_EE_STF_ELF;
    use express_zkvm::{ProverInput, ZKVMHost, ZKVMVerifier};

    const ENCODED_PROVER_INPUT: &[u8] =
        include_bytes!("../../test-util/el_block_witness_input.bin");

    #[test]
    fn test_reth_stf_guest_code_trace_generation() {
        if cfg!(debug_assertions) {
            panic!("SP1 prover runs in release mode only");
        }

        let input: ELProofInput = bincode::deserialize(ENCODED_PROVER_INPUT).unwrap();
        let prover = SP1Host::init(GUEST_EVM_EE_STF_ELF.into(), Default::default());

        let mut prover_input = ProverInput::new();
        prover_input.write(input);

        let (proof, _) = prover
            .prove(&prover_input)
            .expect("Failed to generate proof");

        SP1Verifier::extract_public_output::<ELProofPublicParams>(&proof)
            .expect("Failed to extract public outputs");
    }
}