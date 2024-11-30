mod air;
mod columns;
mod trace;

/// Implements the Poseidon2 permutation operation.
#[derive(Default)]
pub struct Poseidon2PermuteChip;

impl Poseidon2PermuteChip {
    pub const fn new() -> Self {
        Self {}
    }
}

#[cfg(test)]
pub mod permute_tests {
    use sp1_core_executor::{syscalls::SyscallCode, Executor, Instruction, Opcode, Program};
    use sp1_stark::{CpuProver, SP1CoreOpts};

    use crate::utils::{
        self, run_test,
        tests::{POSEIDON2_ELF, POSEIDON2_PERMUTE_ELF},
    };

    pub fn poseidon2_permute_program() -> Program {
        let input_ptr = 100;
        let output_ptr = 1000;
        let mut instructions = vec![Instruction::new(Opcode::ADD, 29, 0, 1, false, true)];
        for i in 0..16 {
            instructions.extend(vec![
                Instruction::new(Opcode::ADD, 30, 0, input_ptr + i * 4, false, true),
                Instruction::new(Opcode::SW, 29, 30, 0, false, true),
            ]);
        }
        instructions.extend(vec![
            Instruction::new(Opcode::ADD, 5, 0, SyscallCode::POSEIDON2_PERMUTE as u32, false, true),
            Instruction::new(Opcode::ADD, 10, 0, input_ptr, false, true),
            Instruction::new(Opcode::ADD, 11, 0, output_ptr, false, true),
            Instruction::new(Opcode::ECALL, 5, 10, 11, false, false),
        ]);

        Program::new(instructions, 0, 0)
    }

    #[test]
    pub fn test_poseidon2_permute_program_execute() {
        utils::setup_logger();
        let program = poseidon2_permute_program();
        let mut runtime = Executor::new(program, SP1CoreOpts::default());
        runtime.run().unwrap();
    }

    #[test]
    fn test_poseidon2_permute_prove_babybear() {
        utils::setup_logger();
        let program = poseidon2_permute_program();
        run_test::<CpuProver<_, _>>(program).unwrap();
    }

    #[test]
    fn test_poseidon2_permute_program_prove() {
        utils::setup_logger();
        let program = Program::from(POSEIDON2_PERMUTE_ELF).unwrap();
        run_test::<CpuProver<_, _>>(program).unwrap();
    }

    #[test]
    fn test_poseidon2_hash_program_prove() {
        utils::setup_logger();
        let program = Program::from(POSEIDON2_ELF).unwrap();
        run_test::<CpuProver<_, _>>(program).unwrap();
    }
}
