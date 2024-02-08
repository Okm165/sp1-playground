use crate::cpu::{MemoryReadRecord, MemoryWriteRecord};

///! This module contains the implementation of the `blake3_compress_inner` precompile based on the
/// implementation of the `blake3` hash function in Plonky3.
mod air;
mod columns;
mod compress_inner;
mod execute;
mod mix;
mod round;
mod trace;

pub(crate) const BLOCK_SIZE: usize = 16;
pub(crate) const BLOCK_LEN_SIZE: usize = 16;
pub(crate) const CV_SIZE: usize = 8;
pub(crate) const COUNTER_SIZE: usize = 2;
pub(crate) const FLAGS_SIZE: usize = 1;

/// The number of `Word`s in the input of the compress inner operation.
pub(crate) const INPUT_SIZE: usize =
    BLOCK_SIZE + BLOCK_LEN_SIZE + CV_SIZE + COUNTER_SIZE + FLAGS_SIZE;

pub(crate) const OUTPUT_SIZE: usize = BLOCK_SIZE;

/// The number of times we call `round` in the compress inner operation.
pub(crate) const ROUND_COUNT: usize = 7;

/// The number of times we call `g` in the compress inner operation.
pub(crate) const OPERATION_COUNT: usize = 7;

#[derive(Debug, Clone, Copy)]
pub struct Blake3CompressInnerEvent {
    pub clk: u32,
    pub state_ptr: u32,
    pub state_reads: [[[MemoryReadRecord; INPUT_SIZE]; OPERATION_COUNT]; ROUND_COUNT],
    pub state_writes: [[[MemoryWriteRecord; OUTPUT_SIZE]; OPERATION_COUNT]; ROUND_COUNT],
}

pub struct Blake3CompressInnerChip {}

impl Blake3CompressInnerChip {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(test)]
pub mod compress_tests {
    use crate::runtime::Instruction;
    use crate::runtime::Opcode;
    use crate::runtime::Syscall;
    use crate::utils::prove;
    use crate::utils::setup_logger;
    use crate::Program;

    use super::INPUT_SIZE;

    pub fn blake3_compress_internal_program() -> Program {
        let w_ptr = 100;
        let mut instructions = vec![];

        for i in 0..INPUT_SIZE {
            // Store 1000 + i in memory for the i-th word of the state. 1000 + i is an arbitrary
            // number that is easy to spot while debugging.
            instructions.extend(vec![
                Instruction::new(Opcode::ADD, 29, 0, 1000 + i as u32, false, true),
                Instruction::new(Opcode::ADD, 30, 0, w_ptr + i as u32 * 4, false, true),
                Instruction::new(Opcode::SW, 29, 30, 0, false, true),
            ]);
        }
        instructions.extend(vec![
            Instruction::new(
                Opcode::ADD,
                5,
                0,
                Syscall::BLAKE3_COMPRESS_INNER as u32,
                false,
                true,
            ),
            Instruction::new(Opcode::ADD, 10, 0, w_ptr, false, true),
            Instruction::new(Opcode::ECALL, 10, 5, 0, false, true),
        ]);
        Program::new(instructions, 0, 0)
    }

    #[test]
    fn prove_babybear() {
        setup_logger();
        let program = blake3_compress_internal_program();
        prove(program);
    }

    // TODO: Create something like this for blake3.
    // #[test]
    // fn test_poseidon2_external_1_simple() {
    //     setup_logger();
    //     let program = Program::from(POSEIDON2_EXTERNAL_1_ELF);
    //     prove(program);
    // }
}
