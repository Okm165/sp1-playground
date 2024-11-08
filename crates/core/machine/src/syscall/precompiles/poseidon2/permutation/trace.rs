use std::borrow::BorrowMut;

use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use sp1_core_executor::{
    events::{ByteRecord, ShaCompressEvent},
    ExecutionRecord, Program,
};
use sp1_stark::air::MachineAir;

use crate::syscall::precompiles::poseidon2::permutation::columns::Poseidon2PermCols;

use super::{columns::NUM_POSEIDON2PERM_COLS, Poseidon2PermChip};

impl<F: PrimeField32> MachineAir<F> for Poseidon2PermChip {
    type Record = ExecutionRecord;

    type Program = Program;

    fn name(&self) -> String {
        "Poseidon2Perm".to_string()
    }

    fn generate_trace(
        &self,
        input: &ExecutionRecord,
        _: &mut ExecutionRecord,
    ) -> RowMajorMatrix<F> {
        todo!()
    }

    fn generate_dependencies(&self, input: &Self::Record, output: &mut Self::Record) {
        todo!()
    }

    fn included(&self, shard: &Self::Record) -> bool {
        todo!()
    }
}

impl Poseidon2PermChip {
    fn event_to_rows<F: PrimeField32>(
        &self,
        event: &ShaCompressEvent,
        rows: &mut Option<Vec<[F; NUM_POSEIDON2PERM_COLS]>>,
        blu: &mut impl ByteRecord,
    ) {
        let shard = event.shard;

        let mut row = [F::zero(); NUM_POSEIDON2PERM_COLS];
        let cols: &mut Poseidon2PermCols<F> = row.as_mut_slice().borrow_mut();

        cols.shard = F::from_canonical_u32(event.shard);
        cols.clk = F::from_canonical_u32(event.clk);

        todo!()
    }
}
