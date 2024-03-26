#![allow(
    clippy::eq_op,
    clippy::new_without_default,
    clippy::field_reassign_with_default,
    clippy::unnecessary_cast,
    clippy::cast_abs_to_unsigned,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::unnecessary_unwrap,
    clippy::default_constructed_unit_structs,
    clippy::box_default,
    incomplete_features
)]
#![feature(generic_const_exprs)]

extern crate alloc;

pub mod air;
pub mod alu;
pub mod bytes;
pub mod cpu;
pub mod disassembler;
pub mod io;
pub mod lookup;
pub mod memory;
pub mod operations;
pub mod program;
pub mod runtime;
pub mod stark;
pub mod syscall;
pub mod utils;

pub use io::*;

use crate::stark::RiscvAir;
use anyhow::Result;
use runtime::{Program, Runtime};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use stark::{Com, OpeningProof, PcsProverData, ProgramVerificationError, Proof, ShardMainData};
use stark::{StarkGenericConfig, Val};
use std::fs;
use utils::{prove_core, run_and_prove, BabyBearPoseidon2};

/// A prover that can prove RISCV ELFs.
pub struct SP1Prover;

/// A verifier that can verify proofs generated by `SP1Prover`.
pub struct SP1Verifier;

/// A proof of a RISCV ELF execution with given inputs and outputs.
#[derive(Serialize, Deserialize)]
pub struct SP1ProofWithIO<SC: StarkGenericConfig + Serialize + DeserializeOwned> {
    #[serde(with = "proof_serde")]
    pub proof: Proof<SC>,
    pub stdin: SP1Stdin,
    pub stdout: SP1Stdout,
}

impl SP1Prover {
    /// Executes the elf with the given inputs and returns the output.
    pub fn execute(elf: &[u8], stdin: SP1Stdin) -> Result<SP1Stdout> {
        let program = Program::from(elf);
        let mut runtime = Runtime::new(program);
        runtime.write_stdin_slice(&stdin.buffer.data);
        runtime.run();
        Ok(SP1Stdout::from(&runtime.state.output_stream))
    }

    /// Generate a proof for the execution of the ELF with the given public inputs.
    pub fn prove(elf: &[u8], stdin: SP1Stdin) -> Result<SP1ProofWithIO<BabyBearPoseidon2>> {
        let config = BabyBearPoseidon2::new();

        let program = Program::from(elf);
        let (proof, stdout) = run_and_prove(program, &stdin.buffer.data, config);
        let stdout = SP1Stdout::from(&stdout);
        Ok(SP1ProofWithIO {
            proof,
            stdin,
            stdout,
        })
    }

    /// Generate a proof for the execution of the ELF with the given public inputs and a custom config.
    pub fn prove_with_config<SC: StarkGenericConfig>(
        elf: &[u8],
        stdin: SP1Stdin,
        config: SC,
    ) -> Result<SP1ProofWithIO<SC>>
    where
        SC: StarkGenericConfig,
        SC::Challenger: Clone,
        OpeningProof<SC>: Send + Sync,
        Com<SC>: Send + Sync,
        PcsProverData<SC>: Send + Sync,
        ShardMainData<SC>: Serialize + DeserializeOwned,
        Val<SC>: p3_field::PrimeField32,
    {
        let program = Program::from(elf);
        let mut runtime = Runtime::new(program);
        runtime.write_stdin_slice(&stdin.buffer.data);
        runtime.run();
        let stdout = SP1Stdout::from(&runtime.state.output_stream);
        let proof = prove_core(config, runtime);
        Ok(SP1ProofWithIO {
            proof,
            stdin,
            stdout,
        })
    }
}

impl SP1Verifier {
    /// Verify a proof generated by `SP1Prover`.
    pub fn verify(
        elf: &[u8],
        proof: &SP1ProofWithIO<BabyBearPoseidon2>,
    ) -> Result<(), ProgramVerificationError> {
        let config = BabyBearPoseidon2::new();
        let mut challenger = config.challenger();
        let machine = RiscvAir::machine(config);
        let (_, vk) = machine.setup(&Program::from(elf));
        tracing::info_span!("verify")
            .in_scope(|| machine.verify(&vk, &proof.proof, &mut challenger))
    }

    /// Verify a proof generated by `SP1Prover` with a custom config.
    pub fn verify_with_config<SC: StarkGenericConfig>(
        elf: &[u8],
        proof: &SP1ProofWithIO<SC>,
        config: SC,
    ) -> Result<(), ProgramVerificationError>
    where
        SC: StarkGenericConfig,
        SC::Challenger: Clone,
        OpeningProof<SC>: Send + Sync,
        Com<SC>: Send + Sync,
        PcsProverData<SC>: Send + Sync,
        ShardMainData<SC>: Serialize + DeserializeOwned,
        SC::Val: p3_field::PrimeField32,
    {
        let mut challenger = config.challenger();
        let machine = RiscvAir::machine(config);

        let (_, vk) = machine.setup(&Program::from(elf));
        machine.verify(&vk, &proof.proof, &mut challenger)
    }
}

impl<SC: StarkGenericConfig + Serialize + DeserializeOwned> SP1ProofWithIO<SC> {
    /// Saves the proof as a JSON to the given path.
    pub fn save(&self, path: &str) -> Result<()> {
        let data = serde_json::to_string(self).unwrap();
        fs::write(path, data).unwrap();
        Ok(())
    }
}
