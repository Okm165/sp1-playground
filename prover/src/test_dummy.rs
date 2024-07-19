#[cfg(test)]
mod tests {
    #[test]
    fn test_dummy_circuit() {
        use p3_baby_bear::DiffusionMatrixBabyBear;
        use p3_field::AbstractExtensionField;
        use rand::{rngs::StdRng, Rng, SeedableRng};
        use sp1_core::{
            stark::{Chip, StarkGenericConfig, StarkMachine, PROOF_MAX_NUM_PVS},
            utils::{run_test_machine, setup_logger, BabyBearPoseidon2Inner},
        };
        use sp1_recursion_core_v2::{
            alu_base::BaseAluChip, alu_ext::ExtAluChip, exp_reverse_bits::ExpReverseBitsLenChip,
            fri_fold::FriFoldChip, machine::RecursionAir, mem::MemoryChip,
            poseidon2_wide::Poseidon2WideChip, RecursionProgram, Runtime,
        };

        use sp1_recursion_compiler::{
            asm::{AsmBuilder, AsmConfig},
            circuit::AsmCompiler,
            ir::*,
        };

        const DEGREE: usize = 3;

        type SC = BabyBearPoseidon2Inner;
        type F = <SC as StarkGenericConfig>::Val;
        type EF = <SC as StarkGenericConfig>::Challenge;
        // type A = RecursionAir<F, DEGREE>;

        setup_logger();

        // To aid in testing.
        const SCALE: usize = 1;
        const FIELD_OPERATIONS: usize = 451653 * SCALE;
        const EXTENSION_OPERATIONS: usize = 82903 * SCALE;
        const POSEIDON_OPERATIONS: usize = 34697 * SCALE;
        const EXP_REVERSE_BITS_LEN_OPERATIONS: usize = 35200 * SCALE;
        const FRI_FOLD_OPERATIONS: usize = 152800 * SCALE;

        let mut builder = AsmBuilder::<F, EF>::default();

        let mut rng = StdRng::seed_from_u64(0xFEB29).sample_iter(rand::distributions::Standard);
        let mut random_felt = move || -> F { rng.next().unwrap() };
        let mut rng =
            StdRng::seed_from_u64(0x0451).sample_iter::<[F; 4], _>(rand::distributions::Standard);
        let mut random_ext = move || EF::from_base_slice(&rng.next().unwrap());

        for _ in 0..FIELD_OPERATIONS {
            let a: Felt<_> = builder.eval(random_felt());
            let b: Felt<_> = builder.eval(random_felt());
            let _: Felt<_> = builder.eval(a + b);
        }
        for _ in 0..EXTENSION_OPERATIONS {
            let a: Ext<_, _> = builder.eval(random_ext().cons());
            let b: Ext<_, _> = builder.eval(random_ext().cons());
            let _: Ext<_, _> = builder.eval(a + b);
        }

        let operations = builder.operations;
        let mut compiler = AsmCompiler::<AsmConfig<F, EF>>::default();
        let instructions = compiler.compile(operations);
        let program = RecursionProgram { instructions };
        let mut runtime = Runtime::<F, EF, DiffusionMatrixBabyBear>::new(
            &program,
            BabyBearPoseidon2Inner::new().perm,
        );
        runtime.run();

        let config = SC::default();
        let chips: Vec<Chip<F, RecursionAir<F, DEGREE>>> = vec![
            RecursionAir::Memory(MemoryChip::default()),
            RecursionAir::BaseAlu(BaseAluChip::default()),
            RecursionAir::ExtAlu(ExtAluChip::default()),
            // RecursionAir::Poseidon2Skinny(
            //     sp1_recursion_core_v2::poseidon2_skinny::Poseidon2SkinnyChip::<DEGREE> {
            //         fixed_log2_rows: Some(((POSEIDON_OPERATIONS * 10 - 1).ilog2() + 1) as usize),
            //         pad: true,
            //     },
            // ),
            RecursionAir::Poseidon2Wide(Poseidon2WideChip::<DEGREE> {
                fixed_log2_rows: Some(((POSEIDON_OPERATIONS - 1).ilog2() + 1) as usize),
                pad: true,
            }),
            RecursionAir::ExpReverseBitsLen(ExpReverseBitsLenChip::<DEGREE> {
                fixed_log2_rows: Some(((EXP_REVERSE_BITS_LEN_OPERATIONS - 1).ilog2() + 1) as usize),
                pad: true,
            }),
            RecursionAir::FriFold(FriFoldChip::<DEGREE> {
                fixed_log2_rows: Some(((FRI_FOLD_OPERATIONS - 1).ilog2() + 1) as usize),
                pad: true,
            }),
        ]
        .into_iter()
        .map(Chip::new)
        .collect();
        let machine = StarkMachine::new(config, chips, PROOF_MAX_NUM_PVS);

        // let machine = A::machine(config);
        let (pk, vk) = machine.setup(&program);
        let result =
            run_test_machine(vec![runtime.record], machine, pk, vk.clone()).expect("should verify");

        tracing::info!("num shard proofs: {}", result.shard_proofs.len());
    }
}
