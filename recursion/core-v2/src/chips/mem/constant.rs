use core::borrow::Borrow;
use itertools::Itertools;
use p3_air::{Air, BaseAir, PairBuilder};
use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;
use sp1_core::air::MachineAir;
use sp1_core::utils::pad_to_power_of_two;
use sp1_derive::AlignedBorrow;
use std::{borrow::BorrowMut, iter::zip, marker::PhantomData};

use crate::{builder::SP1RecursionAirBuilder, *};

use super::MemoryAccessCols;

pub const NUM_MEM_ENTRIES_PER_ROW: usize = 16;

#[derive(Default)]
pub struct MemoryChip<F> {
    _data: PhantomData<F>,
}

pub const NUM_MEM_INIT_COLS: usize = core::mem::size_of::<MemoryCols<u8>>();

#[derive(AlignedBorrow, Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryCols<F: Copy> {
    _nothing: F,
}

pub const NUM_MEM_PREPROCESSED_INIT_COLS: usize =
    core::mem::size_of::<MemoryPreprocessedCols<u8>>();

#[derive(AlignedBorrow, Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryPreprocessedCols<F: Copy> {
    values_and_accesses: [(Block<F>, MemoryAccessCols<F>); NUM_MEM_ENTRIES_PER_ROW],
}
impl<F: Send + Sync> BaseAir<F> for MemoryChip<F> {
    fn width(&self) -> usize {
        NUM_MEM_INIT_COLS
    }
}

impl<F: PrimeField32> MachineAir<F> for MemoryChip<F> {
    type Record = crate::ExecutionRecord<F>;

    type Program = crate::RecursionProgram<F>;

    fn name(&self) -> String {
        "Memory Constants".to_string()
    }
    fn preprocessed_width(&self) -> usize {
        NUM_MEM_PREPROCESSED_INIT_COLS
    }

    fn generate_preprocessed_trace(&self, program: &Self::Program) -> Option<RowMajorMatrix<F>> {
        let rows = program
            .instructions
            .iter()
            .filter_map(|instruction| match instruction {
                Instruction::Mem(MemInstr {
                    addrs,
                    vals,
                    mult,
                    kind,
                }) => {
                    let mult = mult.to_owned();
                    let mult = match kind {
                        MemAccessKind::Read => -mult,
                        MemAccessKind::Write => mult,
                    };

                    Some((
                        vals.inner,
                        MemoryAccessCols {
                            addr: addrs.inner,
                            mult,
                        },
                    ))
                }
                _ => None,
            })
            .chunks(NUM_MEM_ENTRIES_PER_ROW)
            .into_iter()
            .map(|row_vs_as| {
                let mut row = [F::zero(); NUM_MEM_PREPROCESSED_INIT_COLS];
                let cols: &mut MemoryPreprocessedCols<_> = row.as_mut_slice().borrow_mut();
                for (cell, access) in zip(&mut cols.values_and_accesses, row_vs_as) {
                    *cell = access;
                }
                row
            })
            .collect::<Vec<_>>();

        // Convert the trace to a row major matrix.
        let mut trace = RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_MEM_PREPROCESSED_INIT_COLS,
        );

        // Pad the trace to a power of two.
        pad_to_power_of_two::<NUM_MEM_PREPROCESSED_INIT_COLS, F>(&mut trace.values);

        Some(trace)
    }

    fn generate_dependencies(&self, _: &Self::Record, _: &mut Self::Record) {
        // This is a no-op.
    }

    fn generate_trace(&self, input: &Self::Record, _: &mut Self::Record) -> RowMajorMatrix<F> {
        let rows = std::iter::repeat([F::zero(); NUM_MEM_INIT_COLS])
            .take(input.mem_events.len())
            .collect::<Vec<_>>();

        // Convert the trace to a row major matrix.
        let mut trace = RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_MEM_INIT_COLS,
        );

        // Pad the trace to a power of two.
        pad_to_power_of_two::<NUM_MEM_INIT_COLS, F>(&mut trace.values);

        trace
    }

    fn included(&self, _record: &Self::Record) -> bool {
        true
    }
}

impl<AB> Air<AB> for MemoryChip<AB::F>
where
    AB: SP1RecursionAirBuilder + PairBuilder,
{
    fn eval(&self, builder: &mut AB) {
        let prep = builder.preprocessed();
        let prep_local = prep.row_slice(0);
        let prep_local: &MemoryPreprocessedCols<AB::Var> = (*prep_local).borrow();

        for (value, access) in prep_local.values_and_accesses {
            builder.send_block(access.addr, value, access.mult);
        }
    }
}

#[cfg(test)]
mod tests {
    use machine::RecursionAir;
    use p3_baby_bear::{BabyBear, DiffusionMatrixBabyBear};
    use p3_field::AbstractField;
    use p3_matrix::dense::RowMajorMatrix;

    use sp1_core::{
        air::MachineAir,
        stark::StarkGenericConfig,
        utils::{run_test_machine, BabyBearPoseidon2Inner},
    };
    use sp1_recursion_core::stark::config::BabyBearPoseidon2Outer;

    use super::*;

    use crate::runtime::instruction as instr;

    type SC = BabyBearPoseidon2Outer;
    type F = <SC as StarkGenericConfig>::Val;
    type EF = <SC as StarkGenericConfig>::Challenge;
    type A = RecursionAir<F, 3, 1>;

    pub fn prove_program(program: RecursionProgram<F>) {
        let mut runtime = Runtime::<F, EF, DiffusionMatrixBabyBear>::new(
            &program,
            BabyBearPoseidon2Inner::new().perm,
        );
        runtime.run().unwrap();

        let config = SC::new();
        let machine = A::machine(config);
        let (pk, vk) = machine.setup(&program);
        let result = run_test_machine(vec![runtime.record], machine, pk, vk);
        if let Err(e) = result {
            panic!("Verification failed: {:?}", e);
        }
    }

    #[test]
    pub fn generate_trace() {
        let shard = ExecutionRecord::<BabyBear> {
            mem_events: vec![
                MemEvent {
                    inner: BabyBear::one().into(),
                },
                MemEvent {
                    inner: BabyBear::one().into(),
                },
            ],
            ..Default::default()
        };
        let chip = MemoryChip::default();
        let trace: RowMajorMatrix<BabyBear> =
            chip.generate_trace(&shard, &mut ExecutionRecord::default());
        println!("{:?}", trace.values)
    }

    #[test]
    pub fn prove_basic_mem() {
        prove_program(RecursionProgram {
            instructions: vec![
                instr::mem(MemAccessKind::Write, 1, 1, 2),
                instr::mem(MemAccessKind::Read, 1, 1, 2),
            ],
            traces: Default::default(),
        });
    }

    #[test]
    #[should_panic]
    pub fn basic_mem_bad_mult() {
        prove_program(RecursionProgram {
            instructions: vec![
                instr::mem(MemAccessKind::Write, 1, 1, 2),
                instr::mem(MemAccessKind::Read, 999, 1, 2),
            ],
            traces: Default::default(),
        });
    }

    #[test]
    #[should_panic]
    pub fn basic_mem_bad_address() {
        prove_program(RecursionProgram {
            instructions: vec![
                instr::mem(MemAccessKind::Write, 1, 1, 2),
                instr::mem(MemAccessKind::Read, 1, 999, 2),
            ],
            traces: Default::default(),
        });
    }

    #[test]
    #[should_panic]
    pub fn basic_mem_bad_value() {
        prove_program(RecursionProgram {
            instructions: vec![
                instr::mem(MemAccessKind::Write, 1, 1, 2),
                instr::mem(MemAccessKind::Read, 1, 1, 999),
            ],
            traces: Default::default(),
        });
    }
}
