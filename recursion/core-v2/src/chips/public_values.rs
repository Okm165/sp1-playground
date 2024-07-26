use std::borrow::{Borrow, BorrowMut};

use p3_air::{Air, AirBuilder, BaseAir, PairBuilder};
use p3_field::PrimeField32;
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use sp1_core::{air::MachineAir, utils::pad_rows_fixed};
use sp1_derive::AlignedBorrow;

use crate::{
    builder::SP1RecursionAirBuilder,
    runtime::{Instruction, RecursionProgram},
    ExecutionRecord,
};

use crate::DIGEST_SIZE;

use super::mem::MemoryAccessCols;

pub const NUM_PUBLIC_VALUES_COLS: usize = core::mem::size_of::<PublicValuesCols<u8>>();
pub const NUM_PUBLIC_VALUES_PREPROCESSED_COLS: usize =
    core::mem::size_of::<PublicValuesPreprocessedCols<u8>>();

#[derive(Default)]
pub struct PublicValuesChip {}

/// The preprocessed columns for the CommitPVHash instruction.
#[derive(AlignedBorrow, Debug, Clone, Copy)]
#[repr(C)]
pub struct PublicValuesPreprocessedCols<T: Copy> {
    pub pv_idx: [T; DIGEST_SIZE],
    pub pv_mem: MemoryAccessCols<T>,
}

/// The cols for a CommitPVHash invocation.
#[derive(AlignedBorrow, Debug, Clone, Copy)]
#[repr(C)]
pub struct PublicValuesCols<T: Copy> {
    pub pv_element: T,
}

impl<F> BaseAir<F> for PublicValuesChip {
    fn width(&self) -> usize {
        NUM_PUBLIC_VALUES_COLS
    }
}

impl<F: PrimeField32> MachineAir<F> for PublicValuesChip {
    type Record = ExecutionRecord<F>;

    type Program = RecursionProgram<F>;

    fn name(&self) -> String {
        "PublicValues".to_string()
    }

    fn generate_dependencies(&self, _: &Self::Record, _: &mut Self::Record) {
        // This is a no-op.
    }

    fn preprocessed_width(&self) -> usize {
        NUM_PUBLIC_VALUES_PREPROCESSED_COLS
    }

    fn generate_preprocessed_trace(&self, program: &Self::Program) -> Option<RowMajorMatrix<F>> {
        let mut rows: Vec<[F; NUM_PUBLIC_VALUES_PREPROCESSED_COLS]> = Vec::new();
        let commit_pv_hash_instrs = program
            .instructions
            .iter()
            .filter_map(|instruction| {
                if let Instruction::CommitPVHash(instr) = instruction {
                    Some(instr)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if commit_pv_hash_instrs.len() != 1 {
            tracing::warn!("Expected exactly one CommitPVHash instruction.");
        }

        // We only take 1 commit pv hash instruction, since our air only checks for one public values hash.
        for instr in commit_pv_hash_instrs.iter().take(1) {
            for (i, addr) in instr.pv_addrs.iter().enumerate() {
                let mut row: [F; 11] = [F::zero(); NUM_PUBLIC_VALUES_PREPROCESSED_COLS];
                let cols: &mut PublicValuesPreprocessedCols<F> = row.as_mut_slice().borrow_mut();
                cols.pv_idx[i] = F::one();
                cols.pv_mem = MemoryAccessCols {
                    addr: *addr,
                    read_mult: F::one(),
                    write_mult: F::zero(),
                };
                rows.push(row);
            }
        }

        // Pad the preprocessed rows to 8 rows.
        pad_rows_fixed(
            &mut rows,
            || [F::zero(); NUM_PUBLIC_VALUES_PREPROCESSED_COLS],
            Some(3),
        );

        let trace = RowMajorMatrix::new(
            rows.into_iter().flatten().collect(),
            NUM_PUBLIC_VALUES_PREPROCESSED_COLS,
        );
        Some(trace)
    }

    fn generate_trace(
        &self,
        input: &ExecutionRecord<F>,
        _: &mut ExecutionRecord<F>,
    ) -> RowMajorMatrix<F> {
        if input.commit_pv_hash_events.len() != 1 {
            tracing::warn!("Expected exactly one CommitPVHash event.");
        }

        assert!(input.commit_pv_hash_events.len() == 1);

        let mut rows: Vec<[F; NUM_PUBLIC_VALUES_COLS]> = Vec::new();

        // We only take 1 commit pv hash instruction, since our air only checks for one public values hash.
        for event in input.commit_pv_hash_events.iter().take(1) {
            for element in event.pv_hash.iter() {
                let mut row = [F::zero(); NUM_PUBLIC_VALUES_COLS];
                let cols: &mut PublicValuesCols<F> = row.as_mut_slice().borrow_mut();

                cols.pv_element = *element;
                rows.push(row);
            }
        }

        // Pad the trace to 8 rows.
        pad_rows_fixed(&mut rows, || [F::zero(); NUM_PUBLIC_VALUES_COLS], Some(3));

        // Convert the trace to a row major matrix.
        RowMajorMatrix::new(rows.into_iter().flatten().collect(), NUM_PUBLIC_VALUES_COLS)
    }

    fn included(&self, _record: &Self::Record) -> bool {
        true
    }
}

impl<AB> Air<AB> for PublicValuesChip
where
    AB: SP1RecursionAirBuilder + PairBuilder,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local = main.row_slice(0);
        let local: &PublicValuesCols<AB::Var> = (*local).borrow();
        let prepr = builder.preprocessed();
        let local_prepr = prepr.row_slice(0);
        let local_prepr: &PublicValuesPreprocessedCols<AB::Var> = (*local_prepr).borrow();
        let pv = builder.public_values();
        let pv_elms: [AB::Expr; DIGEST_SIZE] = core::array::from_fn(|i| pv[i].into());

        // Constrain mem read for the public value element.
        builder.receive_single(
            local_prepr.pv_mem.addr,
            local.pv_element,
            local_prepr.pv_mem.read_mult,
        );

        for i in 0..DIGEST_SIZE {
            // Ensure that the public value element is the same for all rows within a fri fold invocation.
            builder
                .when(local_prepr.pv_idx[i])
                .assert_eq(pv_elms[i].clone(), local.pv_element);
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand::Rng;
    use rand::SeedableRng;
    use sp1_core::air::MachineAir;
    use sp1_core::utils::run_test_machine;
    use sp1_core::utils::setup_logger;
    use sp1_core::utils::BabyBearPoseidon2;
    use sp1_core::utils::DIGEST_SIZE;
    use sp1_recursion_core::stark::config::BabyBearPoseidon2Outer;
    use std::array;

    use p3_baby_bear::BabyBear;
    use p3_baby_bear::DiffusionMatrixBabyBear;
    use p3_field::AbstractField;
    use p3_matrix::dense::RowMajorMatrix;
    use sp1_core::stark::StarkGenericConfig;

    use crate::chips::public_values::PublicValuesChip;
    use crate::CommitPVHashEvent;
    use crate::{
        machine::RecursionAir,
        runtime::{instruction as instr, ExecutionRecord},
        MemAccessKind, RecursionProgram, Runtime,
    };

    #[test]
    fn prove_babybear_circuit_public_values() {
        setup_logger();
        type SC = BabyBearPoseidon2Outer;
        type F = <SC as StarkGenericConfig>::Val;
        type EF = <SC as StarkGenericConfig>::Challenge;
        type A = RecursionAir<F, 3, 1>;

        let mut rng = StdRng::seed_from_u64(0xDEADBEEF);
        let mut random_felt = move || -> F { F::from_canonical_u32(rng.gen_range(0..1 << 16)) };
        let random_pv_hash: [F; DIGEST_SIZE] = array::from_fn(|_| random_felt());
        let addr = 0u32;

        let mut instructions = Vec::new();
        // Allocate the memory for the public values hash.
        let public_values_hash_a: [u32; DIGEST_SIZE] = array::from_fn(|i| i as u32 + addr);

        for i in 0..DIGEST_SIZE {
            instructions.push(instr::mem_block(
                MemAccessKind::Write,
                1,
                public_values_hash_a[i] as u32,
                random_pv_hash[i].into(),
            ));
        }

        instructions.push(instr::commit_pv_hash(public_values_hash_a));

        let program = RecursionProgram {
            instructions,
            traces: Default::default(),
        };

        let config = SC::new();

        let mut runtime =
            Runtime::<F, EF, DiffusionMatrixBabyBear>::new(&program, BabyBearPoseidon2::new().perm);
        runtime.run().unwrap();
        let machine = A::machine(config);
        let (pk, vk) = machine.setup(&program);
        let result = run_test_machine(vec![runtime.record], machine, pk, vk);
        if let Err(e) = result {
            panic!("Verification failed: {:?}", e);
        }
    }

    #[test]
    fn generate_public_values_circuit_trace() {
        type F = BabyBear;

        let mut rng = StdRng::seed_from_u64(0xDEADBEEF);
        let mut random_felt = move || -> F { F::from_canonical_u32(rng.gen_range(0..1 << 16)) };
        let random_digest = [random_felt(); DIGEST_SIZE];

        let shard = ExecutionRecord {
            commit_pv_hash_events: vec![CommitPVHashEvent {
                pv_hash: random_digest,
            }],
            ..Default::default()
        };
        let chip = PublicValuesChip::default();
        let trace: RowMajorMatrix<F> = chip.generate_trace(&shard, &mut ExecutionRecord::default());
        println!("{:?}", trace.values)
    }
}
