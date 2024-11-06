use std::borrow::BorrowMut;

use super::{
    columns::{Byte3MultCols, NUM_BYTE3_MULT_COLS, NUM_BYTE3_PREPROCESSED_COLS},
    Byte3Chip,
};
use crate::utils::zeroed_f_vec;
use p3_field::Field;
use p3_matrix::dense::RowMajorMatrix;
use sp1_core_executor::{ExecutionRecord, Program};
use sp1_stark::air::MachineAir;

pub const NUM_ROWS: usize = 1 << 24;

impl<F: Field> MachineAir<F> for Byte3Chip<F> {
    type Record = ExecutionRecord;

    type Program = Program;

    fn name(&self) -> String {
        "Byte3".to_string()
    }

    fn preprocessed_width(&self) -> usize {
        NUM_BYTE3_PREPROCESSED_COLS
    }

    fn generate_preprocessed_trace(&self, _program: &Self::Program) -> Option<RowMajorMatrix<F>> {
        let trace = Self::trace();
        Some(trace)
    }

    fn generate_dependencies(&self, _input: &ExecutionRecord, _output: &mut ExecutionRecord) {
        // Do nothing since this chip has no dependencies.
    }

    fn generate_trace(
        &self,
        input: &ExecutionRecord,
        _output: &mut ExecutionRecord,
    ) -> RowMajorMatrix<F> {
        let mut trace =
            RowMajorMatrix::new(zeroed_f_vec(NUM_BYTE3_MULT_COLS * NUM_ROWS), NUM_BYTE3_MULT_COLS);

        for (_, blu) in input.byte3_lookups.iter() {
            for (lookup, mult) in blu.iter() {
                let row = (((lookup.a as u32) << 16) + ((lookup.b as u32) << 8) + lookup.c as u32)
                    as usize;
                let index = lookup.opcode as usize;

                let cols: &mut Byte3MultCols<F> = trace.row_mut(row).borrow_mut();
                cols.multiplicities[index] += F::from_canonical_usize(*mult);
            }
        }

        trace
    }

    fn included(&self, _shard: &Self::Record) -> bool {
        true
    }
}
