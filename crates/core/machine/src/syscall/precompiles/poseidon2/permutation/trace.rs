use super::{
    columns::{FullRound, PartialRound, Poseidon2PermuteCols, NUM_POSEIDON2_PERMUTE_COLS},
    Poseidon2PermuteChip,
};
use crate::utils::pad_rows_fixed;
use p3_field::PrimeField32;
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use sp1_core_executor::{
    events::{ByteRecord, Poseidon2PermuteEvent, PrecompileEvent},
    syscalls::SyscallCode,
    ExecutionRecord, Program,
};
use sp1_primitives::{
    external_linear_layer, internal_linear_layer, NUM_FULL_ROUNDS, NUM_PARTIAL_ROUNDS,
    RC_16_30_U32, WIDTH,
};
use sp1_stark::{air::MachineAir, MachineRecord};
use std::borrow::BorrowMut;

impl<F: PrimeField32> MachineAir<F> for Poseidon2PermuteChip {
    type Record = ExecutionRecord;

    type Program = Program;

    fn name(&self) -> String {
        "Poseidon2Permute".to_string()
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
                        let event = if let PrecompileEvent::Poseidon2Permute(event) = event {
                            event
                        } else {
                            unreachable!()
                        };
                        let mut row: [F; NUM_POSEIDON2_PERMUTE_COLS] =
                            [F::zero(); NUM_POSEIDON2_PERMUTE_COLS];

                        Poseidon2PermuteChip::event_to_row(
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
            || [F::zero(); NUM_POSEIDON2_PERMUTE_COLS],
            input.fixed_log2_rows::<F, _>(self),
        );

        println!("rows: {:#?}", rows);

        // Convert the trace to a row major matrix.
        let mut trace = RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_POSEIDON2_PERMUTE_COLS,
        );

        // Write the nonces to the trace.
        for i in 0..trace.height() {
            let cols: &mut Poseidon2PermuteCols<F> = trace.values
                [i * NUM_POSEIDON2_PERMUTE_COLS..(i + 1) * NUM_POSEIDON2_PERMUTE_COLS]
                .borrow_mut();
            cols.nonce = F::from_canonical_usize(i);
        }

        println!("trace: {:#?}", trace);

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

impl Poseidon2PermuteChip {
    fn event_to_row<F: PrimeField32>(
        event: &Poseidon2PermuteEvent,
        row: &mut [F; NUM_POSEIDON2_PERMUTE_COLS],
        blu: &mut impl ByteRecord,
    ) {
        let cols: &mut Poseidon2PermuteCols<F> = row.as_mut_slice().borrow_mut();

        // Assign basic values to the columns.
        cols.is_real = F::one();
        cols.shard = F::from_canonical_u32(event.shard);
        cols.clk = F::from_canonical_u32(event.clk);
        cols.input_ptr = F::from_canonical_u32(event.input_ptr);

        // Populate memory columns.
        for (i, read_record) in event.state_read_records.iter().enumerate() {
            cols.input_memory[i].populate_read(event.state_read_records[i], blu);
            blu.add_u8_range_checks(event.shard, &read_record.value.to_le_bytes());
        }

        let mut state: [F; WIDTH] = event
            .state_values
            .iter()
            .step_by(2)
            .cloned()
            .map(F::from_wrapped_u32)
            .collect::<Vec<F>>()
            .try_into()
            .unwrap();

        cols.state = state;

        println!("state: {:?}", state.iter().map(|f| f.as_canonical_u32()));

        // Perform permutation on the state
        external_linear_layer(&mut state);

        println!("external_linear_layer: {:?}", state.iter().map(|f| f.as_canonical_u32()));

        for round in 0..NUM_FULL_ROUNDS / 2 {
            Self::populate_full_round(
                &mut state,
                &mut cols.beginning_full_rounds[round],
                &RC_16_30_U32[round].map(F::from_wrapped_u32),
            );
        }

        println!("populate_full_round: {:?}", state.iter().map(|f| f.as_canonical_u32()));
        println!(
            "cols.beginning_full_rounds.sbox: {:?}",
            cols.beginning_full_rounds
                .iter()
                .flat_map(|f| f
                    .sbox
                    .into_iter()
                    .map(|f| f.as_canonical_u32())
                    .collect::<Vec<u32>>())
                .collect::<Vec<u32>>()
        );

        println!(
            "cols.beginning_full_rounds.post: {:?}",
            cols.beginning_full_rounds
                .iter()
                .flat_map(|f| f
                    .post
                    .into_iter()
                    .map(|f| f.as_canonical_u32())
                    .collect::<Vec<u32>>())
                .collect::<Vec<u32>>()
        );

        for round in 0..NUM_PARTIAL_ROUNDS {
            Self::populate_partial_round(
                &mut state,
                &mut cols.partial_rounds[round],
                &RC_16_30_U32[round].map(F::from_wrapped_u32)[0],
            );
        }

        for round in 0..NUM_FULL_ROUNDS / 2 {
            Self::populate_full_round(
                &mut state,
                &mut cols.ending_full_rounds[round],
                &RC_16_30_U32[round].map(F::from_wrapped_u32),
            );
        }

        for (i, write_record) in event.state_write_records.iter().enumerate() {
            cols.input_memory[i].populate_write(*write_record, blu);
            blu.add_u8_range_checks(event.shard, &write_record.value.to_le_bytes());
        }
    }

    pub fn populate_full_round<F: PrimeField32>(
        state: &mut [F; WIDTH],
        full_round: &mut FullRound<F>,
        round_constants: &[F; WIDTH],
    ) {
        for (i, (s, r)) in state.iter_mut().zip(round_constants.iter()).enumerate() {
            *s = *s + *r;
            Self::populate_sbox(&mut full_round.sbox[i], s);
        }
        external_linear_layer(state);
        for (post_i, state_i) in full_round.post.iter_mut().zip(state) {
            *post_i = *state_i;
        }
    }

    pub fn populate_partial_round<F: PrimeField32>(
        state: &mut [F; WIDTH],
        partial_round: &mut PartialRound<F>,
        round_constant: &F,
    ) {
        state[0] = state[0] + *round_constant;
        Self::populate_sbox(&mut partial_round.sbox, &mut state[0]);

        partial_round.post_sbox = state[0];

        internal_linear_layer(state);
    }

    #[inline]
    pub fn populate_sbox<F: PrimeField32>(sbox: &mut F, x: &mut F) {
        *x = x.exp_const_u64::<7>();
        *sbox = *x;
    }
}
