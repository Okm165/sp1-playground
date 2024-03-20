use core::borrow::{Borrow, BorrowMut};
use core::mem::size_of;
use p3_air::{Air, BaseAir};
use p3_field::PrimeField;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::MatrixRowSlices;
use p3_maybe_rayon::prelude::ParallelIterator;
use p3_maybe_rayon::prelude::ParallelSlice;
use sp1_derive::AlignedBorrow;
use tracing::instrument;

use crate::air::MachineAir;
use crate::air::{SP1AirBuilder, Word};
use crate::operations::AddOperation;
use crate::runtime::{ExecutionRecord, Opcode};
use crate::stark::MachineRecord;
use crate::utils::pad_to_power_of_two;

/// The number of main trace columns for `AddSubChip`.
pub const NUM_ADD_SUB_COLS: usize = size_of::<AddSubCols<u8>>();

/// A chip that implements addition for the opcode ADD.
#[derive(Default)]
pub struct AddSubChip;

/// The column layout for the chip.
#[derive(AlignedBorrow, Default, Clone, Copy)]
#[repr(C)]
pub struct AddSubCols<T> {
    /// Boolean to indicate whether the row is for an add operation.
    pub is_add: T,
    /// Boolean to indicate whether the row is for a sub operation.
    pub is_sub: T,

    /// Instance of `AddOperation` to handle addition logic in `AddSubChip`'s ALU operations.
    pub add_operation: AddOperation<T>,

    /// The first input operand.
    pub b: Word<T>,

    /// The second input operand.
    pub c: Word<T>,
}

impl<F: PrimeField> MachineAir<F> for AddSubChip {
    type Record = ExecutionRecord;

    fn name(&self) -> String {
        "AddSub".to_string()
    }

    #[instrument(name = "generate add_sub trace", level = "debug", skip_all)]
    fn generate_trace(
        &self,
        input: &ExecutionRecord,
        output: &mut ExecutionRecord,
    ) -> RowMajorMatrix<F> {
        // Generate the rows for the trace.
        let chunk_size = std::cmp::max(
            (input.add_events.len() + input.sub_events.len()) / num_cpus::get(),
            1,
        );
        let merged_events = input
            .add_events
            .iter()
            .chain(input.sub_events.iter())
            .collect::<Vec<_>>();

        let rows_and_records = merged_events
            .par_chunks(chunk_size)
            .map(|events| {
                let mut record = ExecutionRecord::default();
                let rows = events
                    .iter()
                    .map(|event| {
                        let mut row = [F::zero(); NUM_ADD_SUB_COLS];
                        let cols: &mut AddSubCols<F> = row.as_mut_slice().borrow_mut();
                        let is_add = event.opcode == Opcode::ADD;
                        cols.is_add = F::from_bool(is_add);
                        cols.is_sub = F::from_bool(!is_add);

                        // SUB is basically an ADD with a re-arrangment of the operands.
                        // E.g. ADD verifies a = b + c, and SUB verifies b = a + c.
                        // We assign the operands for the add operation depending on the opcode.
                        let operand_1 = if is_add { event.b } else { event.a };
                        let operand_2 = event.c;

                        cols.add_operation
                            .populate(&mut record, operand_1, operand_2);
                        cols.b = Word::from(operand_1);
                        cols.c = Word::from(operand_2);
                        row
                    })
                    .collect::<Vec<_>>();
                (rows, record)
            })
            .collect::<Vec<_>>();

        let mut rows: Vec<[F; NUM_ADD_SUB_COLS]> = vec![];
        for mut row_and_record in rows_and_records {
            rows.extend(row_and_record.0);
            output.append(&mut row_and_record.1);
        }

        // Convert the trace to a row major matrix.
        let mut trace = RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_ADD_SUB_COLS,
        );

        // Pad the trace to a power of two.
        pad_to_power_of_two::<NUM_ADD_SUB_COLS, F>(&mut trace.values);

        trace
    }

    fn included(&self, shard: &Self::Record) -> bool {
        !shard.add_events.is_empty() || !shard.sub_events.is_empty()
    }
}

impl<F> BaseAir<F> for AddSubChip {
    fn width(&self) -> usize {
        NUM_ADD_SUB_COLS
    }
}

impl<AB> Air<AB> for AddSubChip
where
    AB: SP1AirBuilder,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local: &AddSubCols<AB::Var> = main.row_slice(0).borrow();

        builder.assert_bool(local.is_add);
        builder.assert_bool(local.is_sub);
        let is_real = local.is_add + local.is_sub;
        builder.assert_bool(is_real.clone());

        // Evaluate the addition operation.
        AddOperation::<AB::F>::eval(builder, local.b, local.c, local.add_operation, is_real);

        // Receive the arguments.
        // For add, the result and operand ordering is a = b + c.
        builder.receive_alu(
            Opcode::ADD.as_field::<AB::F>(),
            local.add_operation.value,
            local.b,
            local.c,
            local.is_add,
        );

        // For sub, the result and operand ordering is b = a + c.
        builder.receive_alu(
            Opcode::SUB.as_field::<AB::F>(),
            local.b,
            local.add_operation.value,
            local.c,
            local.is_sub,
        );

        // Degree 3 constraint to avoid "OodEvaluationMismatch".
        builder.assert_zero(
            local.b[0] * local.b[0] * local.c[0] - local.b[0] * local.b[0] * local.c[0],
        );
    }
}

#[cfg(test)]
mod tests {
    use p3_baby_bear::BabyBear;
    use p3_matrix::dense::RowMajorMatrix;

    use crate::{
        air::MachineAir,
        stark::StarkGenericConfig,
        utils::{uni_stark_prove as prove, uni_stark_verify as verify},
    };
    use rand::{thread_rng, Rng};

    use super::AddSubChip;
    use crate::{
        alu::AluEvent,
        runtime::{ExecutionRecord, Opcode},
        utils::BabyBearPoseidon2,
    };

    #[test]
    fn generate_trace() {
        let mut shard = ExecutionRecord::default();
        shard.add_events = vec![AluEvent::new(0, Opcode::ADD, 14, 8, 6)];
        let chip = AddSubChip::default();
        let trace: RowMajorMatrix<BabyBear> =
            chip.generate_trace(&shard, &mut ExecutionRecord::default());
        println!("{:?}", trace.values)
    }

    #[test]
    fn prove_babybear() {
        let config = BabyBearPoseidon2::new();
        let mut challenger = config.challenger();

        let mut shard = ExecutionRecord::default();
        for _ in 0..1000 {
            let operand_1 = thread_rng().gen_range(0..u32::MAX);
            let operand_2 = thread_rng().gen_range(0..u32::MAX);
            let result = operand_1.wrapping_add(operand_2);
            shard
                .add_events
                .push(AluEvent::new(0, Opcode::ADD, result, operand_1, operand_2));
        }

        let chip = AddSubChip::default();
        let trace: RowMajorMatrix<BabyBear> =
            chip.generate_trace(&shard, &mut ExecutionRecord::default());
        let proof = prove::<BabyBearPoseidon2, _>(&config, &chip, &mut challenger, trace);

        let mut challenger = config.challenger();
        verify(&config, &chip, &mut challenger, &proof).unwrap();
    }
}
