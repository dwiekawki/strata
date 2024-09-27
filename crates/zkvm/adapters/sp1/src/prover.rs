use anyhow::Ok;
use express_zkvm::{Proof, ProverOptions, VerificationKey, ZKVMHost, ZKVMInputBuilder};
use sp1_sdk::ProverClient;

use crate::input::SP1ProofInputBuilder;

/// A host for the `SP1` zkVM that stores the guest program in ELF format.
/// The `SP1Host` is responsible for program execution and proving
#[derive(Clone)]
pub struct SP1Host {
    elf: Vec<u8>,
    prover_options: ProverOptions,
}

impl ZKVMHost for SP1Host {
    type Input<'a> = SP1ProofInputBuilder;
    fn init(guest_code: Vec<u8>, prover_options: ProverOptions) -> Self {
        SP1Host {
            elf: guest_code,
            prover_options,
        }
    }

    fn prove<'a>(
        &self,
        prover_input: <Self::Input<'a> as ZKVMInputBuilder<'a>>::Input,
    ) -> anyhow::Result<(Proof, VerificationKey)> {
        // Init the prover
        if self.prover_options.use_mock_prover {
            std::env::set_var("SP1_PROVER", "mock");
        }
        let client = ProverClient::new();
        let (pk, vk) = client.setup(&self.elf);

        // Start proving
        let mut prover = client.prove(&pk, prover_input);
        if self.prover_options.enable_compression {
            prover = prover.compressed();
        }
        if self.prover_options.stark_to_snark_conversion {
            prover = prover.groth16();
        }

        let proof = prover.run()?;

        // Proof serialization
        let serialized_proof = bincode::serialize(&proof)?;
        let verification_key = bincode::serialize(&vk)?;

        Ok((
            Proof::new(serialized_proof),
            VerificationKey(verification_key),
        ))
    }
}

// NOTE: SP1 prover runs in release mode only; therefore run the tests on release mode only
#[cfg(test)]
#[cfg(not(debug_assertions))]
mod tests {

    use std::{fs::File, io::Write};

    use express_zkvm::ZKVMVerifier;
    use sp1_sdk::{HashableKey, SP1VerifyingKey};

    use super::*;
    use crate::SP1Verifier;

    // Adding compiled guest code `TEST_ELF` to save the build time
    // #![no_main]
    // sp1_zkvm::entrypoint!(main);
    // fn main() {
    //     let n = sp1_zkvm::io::read::<u32>();
    //     sp1_zkvm::io::commit(&n);
    // }
    const TEST_ELF: &[u8] = include_bytes!("../tests/elf/riscv32im-succinct-zkvm-elf");

    #[test]
    fn test_mock_prover() {
        let input: u32 = 1;

        let mut prover_input_builder = SP1ProofInputBuilder::new();
        prover_input_builder.write(&input).unwrap();
        let prover_input = prover_input_builder.build().unwrap();

        // assert proof generation works
        let zkvm = SP1Host::init(TEST_ELF.to_vec(), ProverOptions::default());
        let (proof, vk) = zkvm.prove(prover_input).expect("Failed to generate proof");

        // assert proof verification works
        SP1Verifier::verify(&vk, &proof).expect("Proof verification failed");

        // assert public outputs extraction from proof  works
        let out: u32 = SP1Verifier::extract_public_output(&proof).expect(
            "Failed to extract public
    outputs",
        );
        assert_eq!(input, out)
    }

    #[test]
    fn test_mock_prover_with_public_param() {
        let input: u32 = 1;

        let mut prover_input_builder = SP1ProofInputBuilder::new();
        prover_input_builder.write(&input).unwrap();
        let prover_input = prover_input_builder.build().unwrap();

        // assert proof generation works
        let zkvm = SP1Host::init(TEST_ELF.to_vec(), ProverOptions::default());
        let (proof, vk) = zkvm.prove(prover_input).expect("Failed to generate proof");

        // assert proof verification works
        SP1Verifier::verify_with_public_params(&vk, input, &proof)
            .expect("Proof verification failed");
    }

    #[test]
    fn test_groth16_proof_generation() {
        sp1_sdk::utils::setup_logger();

        let input: u32 = 1;

        let prover_input = SP1ProofInputBuilder::new()
            .write(&input)
            .unwrap()
            .build()
            .unwrap();

        // Prover Options to generate Groth16 proof
        let prover_options = ProverOptions {
            enable_compression: false,
            use_mock_prover: false,
            stark_to_snark_conversion: true,
        };
        let zkvm = SP1Host::init(TEST_ELF.to_vec(), prover_options);

        // assert proof generation works
        let (proof, vk) = zkvm.prove(prover_input).expect("Failed to generate proof");

        let vk: SP1VerifyingKey = bincode::deserialize(vk.as_bytes()).unwrap();

        // Note: For the fixed ELF and fixed SP1 version, the vk is fixed
        assert_eq!(
            vk.bytes32(),
            "0x00b01ae596b4e51843484ff71ccbd0dd1a030af70b255e6b9aad50b81d81266f"
        );

        let filename = "proof-groth16.bin";
        let mut file = File::create(filename).unwrap();
        file.write_all(proof.as_bytes()).unwrap();
    }
}