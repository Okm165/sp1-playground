//! RV32IM ELFs used for testing.

use crate::{Instruction, Opcode, Program};

#[must_use]
#[allow(missing_docs)]
pub fn simple_program() -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
        Instruction::new(Opcode::ADD, 30, 0, 37, false, true),
        Instruction::new(Opcode::ADD, 31, 30, 29, false, false),
    ];
    Program::new(instructions, 0, 0)
}

#[must_use]
#[allow(missing_docs)]
pub fn simple_memory_program() -> Program {
    let instructions = vec![
        Instruction::new(Opcode::ADD, 29, 0, 0x12348765, false, true),
        // SW and LW
        Instruction::new(Opcode::SW, 29, 0, 0x27654320, false, true),
        Instruction::new(Opcode::LW, 28, 0, 0x27654320, false, true),
        // LBU
        Instruction::new(Opcode::LBU, 27, 0, 0x27654320, false, true),
        Instruction::new(Opcode::LBU, 26, 0, 0x27654321, false, true),
        Instruction::new(Opcode::LBU, 25, 0, 0x27654322, false, true),
        Instruction::new(Opcode::LBU, 24, 0, 0x27654323, false, true),
        // LB
        Instruction::new(Opcode::LB, 23, 0, 0x27654320, false, true),
        Instruction::new(Opcode::LB, 22, 0, 0x27654321, false, true),
        // LHU
        Instruction::new(Opcode::LHU, 21, 0, 0x27654320, false, true),
        Instruction::new(Opcode::LHU, 20, 0, 0x27654322, false, true),
        // LU
        Instruction::new(Opcode::LH, 19, 0, 0x27654320, false, true),
        Instruction::new(Opcode::LH, 18, 0, 0x27654322, false, true),
        // SB
        Instruction::new(Opcode::ADD, 17, 0, 0x38276525, false, true),
        // Save the value 0x12348765 into address 0x43627530
        Instruction::new(Opcode::SW, 29, 0, 0x43627530, false, true),
        Instruction::new(Opcode::SB, 17, 0, 0x43627530, false, true),
        Instruction::new(Opcode::LW, 16, 0, 0x43627530, false, true),
        Instruction::new(Opcode::SB, 17, 0, 0x43627531, false, true),
        Instruction::new(Opcode::LW, 15, 0, 0x43627530, false, true),
        Instruction::new(Opcode::SB, 17, 0, 0x43627532, false, true),
        Instruction::new(Opcode::LW, 14, 0, 0x43627530, false, true),
        Instruction::new(Opcode::SB, 17, 0, 0x43627533, false, true),
        Instruction::new(Opcode::LW, 13, 0, 0x43627530, false, true),
        // SH
        // Save the value 0x12348765 into address 0x43627530
        Instruction::new(Opcode::SW, 29, 0, 0x43627530, false, true),
        Instruction::new(Opcode::SH, 17, 0, 0x43627530, false, true),
        Instruction::new(Opcode::LW, 12, 0, 0x43627530, false, true),
        Instruction::new(Opcode::SH, 17, 0, 0x43627532, false, true),
        Instruction::new(Opcode::LW, 11, 0, 0x43627530, false, true),
    ];
    Program::new(instructions, 0, 0)
}
