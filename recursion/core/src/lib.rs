pub mod air;
pub mod cpu;
pub mod memory;
pub mod runtime;
pub mod stark;

#[cfg(test)]
pub mod tests {
    use crate::runtime::{ExecutionRecord, Instruction, Opcode, Program, Runtime};
    use crate::stark::RecursionAir;

    use p3_baby_bear::BabyBear;
    use p3_field::{AbstractField, PrimeField32};
    use sp1_core::stark::{LocalProver, StarkGenericConfig};
    use sp1_core::utils::BabyBearPoseidon2;
    use sp1_core::utils::StarkUtils;
    use std::time::Instant;

    pub fn fibonacci_program<F: PrimeField32>() -> Program<F> {
        // .main
        //  imm 0(fp) 1 <-- a = 1
        //  imm 1(fp) 1 <-- b = 1
        //  imm 2(fp) 10 <-- iterations = 10
        // .body:
        //   add 3(fp) 0(fp) 1(fp) <-- tmp = a + b
        //   sw 0(fp) 1(fp) <-- a = b
        //   sw 1(fp) 3(fp) <-- b = tmp
        // . subi 2(fp) 2(fp) 1 <-- iterations -= 1
        //   bne 2(fp) 0 .body <-- if iterations != 0 goto .body
        Program::<F> {
            instructions: vec![
                // .main
                Instruction::new(Opcode::SW, 0, 1, 0, true, true),
                Instruction::new(Opcode::SW, 1, 1, 0, true, true),
                Instruction::new(Opcode::SW, 2, 10, 0, true, true),
                // .body:
                Instruction::new(Opcode::ADD, 3, 0, 1, false, false),
                Instruction::new(Opcode::SW, 0, 1, 0, false, true),
                Instruction::new(Opcode::SW, 1, 3, 0, false, true),
                Instruction::new(Opcode::SUB, 2, 2, 1, false, true),
                Instruction::new(Opcode::BNE, 2, 0, 3, true, true),
            ],
        }
    }

    #[test]
    fn test_fibonacci_execute() {
        let program = fibonacci_program();
        let mut runtime = Runtime::<BabyBear> {
            clk: BabyBear::zero(),
            program,
            fp: BabyBear::zero(),
            pc: BabyBear::zero(),
            memory: vec![BabyBear::zero(); 1024 * 1024],
            record: ExecutionRecord::<BabyBear>::default(),
        };
        runtime.run();
        println!("{:#?}", runtime.record.cpu_events);
        assert_eq!(runtime.memory[1], BabyBear::from_canonical_u32(144));
    }

    #[test]
    fn test_fibonacci_prove() {
        type SC = BabyBearPoseidon2;
        type F = <SC as StarkGenericConfig>::Val;
        let program = fibonacci_program::<F>();

        let mut runtime = Runtime::<F>::new(&program);
        runtime.run();

        let config = SC::new();
        let machine = RecursionAir::machine(config);
        let (pk, vk) = machine.setup(&program);
        let mut challenger = machine.config().challenger();

        let start = Instant::now();
        let proof = machine.prove::<LocalProver<_, _>>(&pk, runtime.record, &mut challenger);
        let duration = start.elapsed().as_secs();

        let mut challenger = machine.config().challenger();
        machine.verify(&vk, &proof, &mut challenger).unwrap();
        println!("proving duration = {}", duration);
    }
}
