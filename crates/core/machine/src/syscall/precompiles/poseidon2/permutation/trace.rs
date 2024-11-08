use super::{
    columns::{FullRound, PartialRound, NUM_POSEIDON2PERM_COLS},
    Poseidon2PermChip,
};
use crate::syscall::precompiles::poseidon2::{
    permutation::columns::Poseidon2PermCols, NUM_FULL_ROUNDS, NUM_PARTIAL_ROUNDS, WIDTH,
};
use crate::utils::pad_rows_fixed;
use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;
use sp1_core_executor::{
    events::{ByteRecord, MemoryRecordEnum, Poseidon2PermEvent, PrecompileEvent},
    syscalls::SyscallCode,
    ExecutionRecord, Program,
};
use sp1_primitives::RC_16_30_U32;
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

        // Generate the trace rows for each event.
        let mut rows = Vec::new();
        for (row, mut record) in rows_and_records {
            rows.extend(row);
            output.append(&mut record);
        }

        pad_rows_fixed(
            &mut rows,
            || [F::zero(); NUM_POSEIDON2PERM_COLS],
            input.fixed_log2_rows::<F, _>(self),
        );

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

        // Assign basic values to the columns.
        cols.is_real = F::one();
        cols.shard = F::from_canonical_u32(event.shard);
        cols.clk = F::from_canonical_u32(event.clk);
        cols.input_ptr = F::from_canonical_u32(event.input_ptr);

        // Populate memory columns.
        for i in 0..WIDTH {
            cols.input_memory[i]
                .populate(MemoryRecordEnum::Write(event.input_memory_records[i]), blu);
            cols.input_range_checker[i].populate(event.input_memory_records[i].prev_value);
            cols.state[i] = F::from_canonical_u32(event.input_memory_records[i].prev_value);
        }

        // Perform permutation on the state
        Self::external_linear_layer(&mut cols.state);

        for round in 0..NUM_FULL_ROUNDS / 2 {
            Self::populate_full_round(
                &mut cols.state,
                &cols.beginning_full_rounds[round],
                &RC_16_30_U32[round].map(F::from_canonical_u32),
            );
        }

        for round in 0..NUM_PARTIAL_ROUNDS {
            Self::populate_partial_round(
                &mut cols.state,
                &cols.partial_rounds[round],
                &RC_16_30_U32[round].map(F::from_canonical_u32)[0],
            );
        }

        for round in 0..NUM_FULL_ROUNDS / 2 {
            Self::populate_full_round(
                &mut cols.state,
                &cols.ending_full_rounds[round],
                &RC_16_30_U32[round].map(F::from_canonical_u32),
            );
        }
    }

    pub fn populate_full_round<F: PrimeField32>(
        state: &mut [F; WIDTH],
        full_round: &FullRound<F>,
        round_constants: &[F; WIDTH],
    ) {
        for (s, r) in state.iter_mut().zip(round_constants.iter()) {
            *s = *s + *r;
            Self::populate_sbox(s);
        }
        Self::external_linear_layer(state);
        for (state_i, post_i) in state.iter_mut().zip(full_round.post) {
            *state_i = post_i;
        }
    }

    pub fn populate_partial_round<F: PrimeField32>(
        state: &mut [F; WIDTH],
        partial_round: &PartialRound<F>,
        round_constant: &F,
    ) {
        state[0] = state[0] + *round_constant;
        Self::populate_sbox(&mut state[0]);

        state[0] = partial_round.post_sbox;

        Self::internal_linear_layer(state);
    }

    #[inline]
    pub fn populate_sbox<F: PrimeField32>(x: &mut F) {
        *x = x.exp_const_u64::<7>();
    }
}
