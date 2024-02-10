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
    clippy::box_default
)]

extern crate alloc;

pub mod air;
pub mod alu;
pub mod bytes;
pub mod chip;
pub mod cpu;
pub mod disassembler;
pub mod field;
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

use anyhow::Result;
use runtime::{Program, Runtime};
use serde::Serialize;
use stark::{ProgramVerificationError, Proof};
use std::fs;
use utils::{prove_core, BabyBearBlake3, StarkUtils};

/// A prover that can prove RISCV ELFs.
pub struct SuccinctProver;

/// A verifier that can verify proofs generated by `SuccinctProver`.
pub struct SuccinctVerifier;

/// A proof of a RISCV ELF execution with given inputs and outputs.
#[derive(Serialize)]
pub struct SuccinctProofWithPublicInputs {
    #[serde(serialize_with = "serialize_proof")]
    pub proof: Proof<BabyBearBlake3>,
    pub stdin: SuccinctStdin,
    pub stdout: SuccinctStdout,
}

impl SuccinctProver {
    /// Executes the elf with the given inputs and returns the output.
    pub fn execute(elf: &[u8], stdin: SuccinctStdin) -> Result<SuccinctStdout> {
        let program = Program::from(elf);
        let mut runtime = Runtime::new(program);
        runtime.write_stdin_slice(&stdin.buffer.data);
        runtime.run();
        Ok(SuccinctStdout::from(&runtime.state.output_stream))
    }

    /// Generate a proof for the execution of the ELF with the given public inputs.
    pub fn prove(elf: &[u8], stdin: SuccinctStdin) -> Result<SuccinctProofWithPublicInputs> {
        let program = Program::from(elf);
        let mut runtime = Runtime::new(program);
        runtime.write_stdin_slice(&stdin.buffer.data);
        runtime.run();
        let proof = prove_core(&mut runtime);
        Ok(SuccinctProofWithPublicInputs {
            proof,
            stdin,
            stdout: SuccinctStdout::from(&runtime.state.output_stream),
        })
    }
}

impl SuccinctVerifier {
    /// Verify a proof generated by `SuccinctProver`.
    #[allow(unused_variables)]
    pub fn verify(
        elf: &[u8],
        proof: &SuccinctProofWithPublicInputs,
    ) -> Result<(), ProgramVerificationError> {
        let config = BabyBearBlake3::new();
        let mut challenger = config.challenger();
        Runtime::verify(&config, &mut challenger, &proof.proof)
    }
}

impl SuccinctProofWithPublicInputs {
    /// Saves the proof as a JSON to the given path.
    pub fn save(&self, path: &str) -> Result<()> {
        let data = serde_json::to_string(self).unwrap();
        fs::write(path, data).unwrap();
        Ok(())
    }
}
