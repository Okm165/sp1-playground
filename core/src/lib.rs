#![allow(
    clippy::eq_op,
    clippy::new_without_default,
    clippy::field_reassign_with_default,
    clippy::unnecessary_cast,
    clippy::cast_abs_to_unsigned,
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::unnecessary_unwrap,
    clippy::default_constructed_unit_structs
)]

extern crate alloc;

pub mod air;
pub mod alu;
pub mod bytes;
pub mod cpu;
pub mod disassembler;
pub mod field;
pub mod lookup;
pub mod memory;
pub mod operations;
pub mod precompiles;
pub mod program;
pub mod runtime;
pub mod stark;
pub mod utils;

use runtime::{Program, Runtime};
use serde::Serialize;
use utils::prove_core;

pub struct SuccinctProver {
    stdin: Vec<u8>,
}

impl SuccinctProver {
    pub fn new() -> Self {
        Self { stdin: Vec::new() }
    }

    pub fn write_stdin<T: Serialize>(&mut self, input: &T) {
        let mut buf = Vec::new();
        bincode::serialize_into(&mut buf, input).expect("serialization failed");
        self.stdin.extend(buf);
    }

    pub fn prove(&self, elf: &[u8]) {
        let program = Program::from(elf);
        let mut runtime = Runtime::new(program);
        runtime.write_stdin_slice(&self.stdin);
        runtime.run();
        prove_core(&mut runtime);
    }
}
