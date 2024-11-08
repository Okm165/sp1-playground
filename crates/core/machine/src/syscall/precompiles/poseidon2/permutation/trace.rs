use super::{columns::NUM_POSEIDON2PERM_COLS, Poseidon2PermChip};
use crate::syscall::precompiles::poseidon2::{permutation::columns::Poseidon2PermCols, WIDTH};
use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;
use sp1_core_executor::{
    events::{ByteRecord, MemoryRecordEnum, Poseidon2PermEvent, PrecompileEvent},
    syscalls::SyscallCode,
    ExecutionRecord, Program,
};
use sp1_primitives::consts::WORD_SIZE;
use sp1_stark::air::MachineAir;
use sp1_stark::MachineRecord;
use std::borrow::BorrowMut;

impl<F: PrimeField32> MachineAir<F> for Poseidon2PermChip {
    type Record = ExecutionRecord;

    type Program = Program;

    fn name(&self) -> String {
        "Poseidon2Perm".to_string()
    }

    fn generate_trace(
        &self,
        input: &ExecutionRecord,
        output: &mut ExecutionRecord,
    ) -> RowMajorMatrix<F> {
        // Generate the trace rows & corresponding records for each chunk of events concurrently.
        let rows_and_records = input
            .get_precompile_events(SyscallCode::POSEIDON2_PERMUTE)
            .chunks(1)
            .map(|events| {
                let mut records = ExecutionRecord::default();
                let mut new_byte_lookup_events = Vec::new();

                let rows = events
                    .iter()
                    .map(|(_, event)| {
                        let event = if let PrecompileEvent::Poseidon2Perm(event) = event {
                            event
                        } else {
                            unreachable!()
                        };
                        let mut row: [F; NUM_POSEIDON2PERM_COLS] =
                            [F::zero(); NUM_POSEIDON2PERM_COLS];

                        Poseidon2PermChip::event_to_row(
                            event,
                            &mut row,
                            &mut new_byte_lookup_events,
                        );

                        row
                    })
                    .collect::<Vec<_>>();
                records.add_byte_lookup_events(new_byte_lookup_events);
                (rows, records)
            })
            .collect::<Vec<_>>();

        //  Generate the trace rows for each event.
        let mut rows = Vec::new();
        for (row, mut record) in rows_and_records {
            rows.extend(row);
            output.append(&mut record);
        }

        // Convert the trace to a row major matrix.
        let mut trace = RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_POSEIDON2PERM_COLS,
        );

        // Write the nonces to the trace.
        for i in 0..trace.height() {
            let cols: &mut Poseidon2PermCols<F> = trace.values
                [i * NUM_POSEIDON2PERM_COLS..(i + 1) * NUM_POSEIDON2PERM_COLS]
                .borrow_mut();
            cols.nonce = F::from_canonical_usize(i);
        }

        trace
    }

    fn included(&self, shard: &Self::Record) -> bool {
        if let Some(shape) = shard.shape.as_ref() {
            shape.included::<F, _>(self)
        } else {
            !shard.get_precompile_events(SyscallCode::POSEIDON2_PERMUTE).is_empty()
        }
    }
}

impl Poseidon2PermChip {
    fn event_to_row<F: PrimeField32>(
        event: &Poseidon2PermEvent,
        row: &mut [F; NUM_POSEIDON2PERM_COLS],
        blu: &mut impl ByteRecord,
    ) {
        let cols: &mut Poseidon2PermCols<F> = row.as_mut_slice().borrow_mut();

        // Decode input
        let input: Vec<F> = event.input.iter().map(|e| F::from_canonical_u32(*e)).collect();

        // Assign basic values to the columns.
        cols.is_real = F::one();
        cols.shard = F::from_canonical_u32(event.shard);
        cols.clk = F::from_canonical_u32(event.clk);
        cols.input_ptr = F::from_canonical_u32(event.input_ptr);

        // Populate memory columns. Q!
        for i in 0..(WIDTH / WORD_SIZE) {
            cols.input_memory[i]
                .populate(MemoryRecordEnum::Write(event.input_memory_records[i]), blu);
        }

        todo!();
    }
}
