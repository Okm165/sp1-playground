use std::borrow::BorrowMut;
use std::cmp::min;

use p3_air::BaseAir;
use p3_field::PrimeField32;
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use sp1_core::{air::MachineAir, utils::pad_rows_fixed};
use sp1_primitives::RC_16_30_U32;
use tracing::instrument;

use crate::poseidon2::{Poseidon2AbsorbEvent, Poseidon2CompressEvent, Poseidon2FinalizeEvent};
use crate::poseidon2_wide::columns::permutation::permutation_mut;
use crate::{
    poseidon2::Poseidon2Event,
    poseidon2_wide::{
        columns::Poseidon2Degree3, external_linear_layer, NUM_EXTERNAL_ROUNDS, WIDTH,
    },
    runtime::{ExecutionRecord, RecursionProgram},
};

use super::RATE;
use super::{
    columns::{Poseidon2Degree9, Poseidon2Mut},
    internal_linear_layer, Poseidon2WideChip, NUM_INTERNAL_ROUNDS,
};

impl<F: PrimeField32, const DEGREE: usize> MachineAir<F> for Poseidon2WideChip<DEGREE> {
    type Record = ExecutionRecord<F>;

    type Program = RecursionProgram<F>;

    fn name(&self) -> String {
        format!("Poseidon2Wide {}", DEGREE)
    }

    fn generate_dependencies(&self, _: &Self::Record, _: &mut Self::Record) {
        // This is a no-op.
    }

    #[instrument(name = "generate poseidon2 wide trace", level = "debug", skip_all, fields(rows = input.poseidon2_events.len()))]
    fn generate_trace(
        &self,
        input: &ExecutionRecord<F>,
        _: &mut ExecutionRecord<F>,
    ) -> RowMajorMatrix<F> {
        let mut rows = Vec::new();

        let num_columns = <Poseidon2WideChip<DEGREE> as BaseAir<F>>::width(self);

        for event in &input.poseidon2_events {
            match event {
                Poseidon2Event::Compress(compress_event) => {
                    assert!(compress_event.left != F::zero());
                    assert!(compress_event.right != F::zero());
                    assert!(compress_event.dst != F::zero());
                    rows.extend(self.populate_compress_event(compress_event, num_columns));
                }

                Poseidon2Event::Absorb(absorb_event) => {
                    assert!(absorb_event.input_ptr != F::zero());
                    rows.extend(self.populate_absorb_event(absorb_event, num_columns));
                }

                Poseidon2Event::Finalize(finalize_event) => {
                    assert!(finalize_event.output_ptr != F::zero());
                    rows.push(self.populate_finalize_event(finalize_event, num_columns));
                }
            }
        }

        // Pad the trace to a power of two.
        pad_rows_fixed(
            &mut rows,
            || vec![F::zero(); num_columns],
            self.fixed_log2_rows,
        );

        // Convert the trace to a row major matrix.
        let trace =
            RowMajorMatrix::new(rows.into_iter().flatten().collect::<Vec<_>>(), num_columns);

        #[cfg(debug_assertions)]
        println!(
            "poseidon2 wide trace dims is width: {:?}, height: {:?}",
            trace.width(),
            trace.height()
        );

        trace
    }

    fn included(&self, record: &Self::Record) -> bool {
        !record.poseidon2_events.is_empty()
    }
}

impl<const DEGREE: usize> Poseidon2WideChip<DEGREE> {
    pub fn convert_mut<'a, 'b: 'a, F: PrimeField32>(
        &self,
        row: &'b mut Vec<F>,
    ) -> Box<dyn Poseidon2Mut<'a, F> + 'a> {
        if DEGREE == 3 {
            let convert: &mut Poseidon2Degree3<F> = row.as_mut_slice().borrow_mut();
            Box::new(convert)
        } else if DEGREE == 9 {
            let convert: &mut Poseidon2Degree9<F> = row.as_mut_slice().borrow_mut();
            Box::new(convert)
        } else {
            panic!("Unsupported degree");
        }
    }

    pub fn populate_compress_event<F: PrimeField32>(
        &self,
        compress_event: &Poseidon2CompressEvent<F>,
        num_columns: usize,
    ) -> Vec<Vec<F>> {
        let mut compress_rows = Vec::new();

        let mut input_row = vec![F::zero(); num_columns];
        {
            let mut cols = self.convert_mut(&mut input_row);
            let control_flow = cols.control_flow_mut();

            control_flow.is_compress = F::one();
            control_flow.is_syscall = F::one();
            control_flow.is_input = F::one();
            control_flow.do_perm = F::one();
        }

        {
            let mut cols = self.convert_mut(&mut input_row);
            let syscall_params = cols.syscall_params_mut().compress_mut();

            syscall_params.clk = compress_event.clk;
            syscall_params.dst_ptr = compress_event.dst;
            syscall_params.left_ptr = compress_event.left;
            syscall_params.right_ptr = compress_event.right;
        }

        {
            let mut cols = self.convert_mut(&mut input_row);
            let memory = cols.memory_mut();

            memory.start_addr = compress_event.left;
            // Populate the first half of the memory inputs in the memory struct.
            for i in 0..WIDTH / 2 {
                memory.memory_slot_used[i] = F::one();
                memory.memory_accesses[i].populate(&compress_event.input_records[i]);
            }
        }

        {
            let mut cols = self.convert_mut(&mut input_row);
            let compress_cols = cols.opcode_workspace_mut().compress_mut();
            compress_cols.start_addr = compress_event.right;

            // Populate the second half of the memory inputs.
            for i in 0..WIDTH / 2 {
                compress_cols.memory_accesses[i]
                    .populate(&compress_event.input_records[i + WIDTH / 2]);
            }
        }

        {
            let mut permutation = permutation_mut::<F, DEGREE>(&mut input_row);

            let (
                external_rounds_state,
                internal_rounds_state,
                internal_rounds_s0,
                mut external_sbox,
                mut internal_sbox,
                output_state,
            ) = permutation.get_cols_mut();

            external_rounds_state[0] = compress_event.input;
            external_linear_layer(&mut external_rounds_state[0]);

            // Apply the first half of external rounds.
            for r in 0..NUM_EXTERNAL_ROUNDS / 2 {
                let next_state =
                    populate_external_round(external_rounds_state, &mut external_sbox, r);
                if r == NUM_EXTERNAL_ROUNDS / 2 - 1 {
                    *internal_rounds_state = next_state;
                } else {
                    external_rounds_state[r + 1] = next_state;
                }
            }

            // Apply the internal rounds.
            external_rounds_state[NUM_EXTERNAL_ROUNDS / 2] = populate_internal_rounds(
                internal_rounds_state,
                internal_rounds_s0,
                &mut internal_sbox,
            );

            // Apply the second half of external rounds.
            for r in NUM_EXTERNAL_ROUNDS / 2..NUM_EXTERNAL_ROUNDS {
                let next_state =
                    populate_external_round(external_rounds_state, &mut external_sbox, r);
                if r == NUM_EXTERNAL_ROUNDS - 1 {
                    for i in 0..WIDTH {
                        output_state[i] = next_state[i];
                        assert_eq!(compress_event.result_records[i].value[0], next_state[i]);
                    }
                } else {
                    external_rounds_state[r + 1] = next_state;
                }
            }
        }

        compress_rows.push(input_row);

        let mut output_row = vec![F::zero(); num_columns];
        {
            let mut cols = self.convert_mut(&mut output_row);
            let control_flow = cols.control_flow_mut();

            control_flow.is_compress = F::one();
            control_flow.is_output = F::one();
            control_flow.is_compress_output = F::one();
        }

        {
            let mut cols = self.convert_mut(&mut output_row);
            let syscall_cols = cols.syscall_params_mut().compress_mut();

            syscall_cols.clk = compress_event.clk;
            syscall_cols.dst_ptr = compress_event.dst;
            syscall_cols.left_ptr = compress_event.left;
            syscall_cols.right_ptr = compress_event.right;
        }

        {
            let mut cols = self.convert_mut(&mut output_row);
            let memory = cols.memory_mut();

            memory.start_addr = compress_event.dst;
            // Populate the first half of the memory inputs in the memory struct.
            for i in 0..WIDTH / 2 {
                memory.memory_slot_used[i] = F::one();
                memory.memory_accesses[i].populate(&compress_event.result_records[i]);
            }
        }

        {
            let mut cols = self.convert_mut(&mut output_row);
            let compress_cols = cols.opcode_workspace_mut().compress_mut();

            compress_cols.start_addr = compress_event.dst + F::from_canonical_usize(WIDTH / 2);
            for i in 0..WIDTH / 2 {
                compress_cols.memory_accesses[i]
                    .populate(&compress_event.result_records[i + WIDTH / 2]);
            }
        }

        compress_rows.push(output_row);
        compress_rows
    }

    pub fn populate_absorb_event<F: PrimeField32>(
        &self,
        absorb_event: &Poseidon2AbsorbEvent<F>,
        num_columns: usize,
    ) -> Vec<Vec<F>> {
        let mut absorb_rows = Vec::new();

        // Handle the first chunk.  It may not be the full hash rate.
        // Create a vec of each absorb row's input size.
        let first_chunk_size = min(
            RATE - absorb_event.hash_state_cursor,
            absorb_event.input_len,
        );
        let mut input_sizes = vec![first_chunk_size];

        let num_rate_sizes = (absorb_event.input_len - first_chunk_size) / RATE;
        input_sizes.extend(vec![RATE; num_rate_sizes]);
        let last_chunk_size = (absorb_event.input_len - first_chunk_size) % RATE;
        if last_chunk_size > 0 {
            input_sizes.push(last_chunk_size);
        }

        let mut state_cursor = absorb_event.hash_state_cursor;
        let mut input_cursor = 0;
        for (row_num, input_size) in input_sizes.iter().enumerate() {
            let mut absorb_row = vec![F::zero(); num_columns];

            {
                let mut cols = self.convert_mut(&mut absorb_row);
                let control_flow = cols.control_flow_mut();

                control_flow.is_absorb = F::one();
                control_flow.is_syscall = F::from_bool(row_num == 0);
                control_flow.is_input = F::one();
                control_flow.do_perm = F::from_bool(state_cursor + input_size == RATE);
            }

            {
                let mut cols = self.convert_mut(&mut absorb_row);
                let syscall_params = cols.syscall_params_mut().absorb_mut();

                syscall_params.clk = absorb_event.clk;
                syscall_params.hash_num = absorb_event.hash_num;
                syscall_params.input_ptr = absorb_event.input_ptr;
                syscall_params.len = F::from_canonical_usize(absorb_event.input_len);
            }

            {
                let mut cols = self.convert_mut(&mut absorb_row);
                let memory = cols.memory_mut();

                // Populate the memory.
                memory.start_addr = absorb_event.input_ptr + F::from_canonical_usize(input_cursor);
                for i in 0..RATE {
                    if i >= state_cursor && (input_cursor < absorb_event.input_len) {
                        memory.memory_slot_used[i] = F::one();
                        memory.memory_accesses[i]
                            .populate(&absorb_event.input_records[input_cursor]);
                        input_cursor += 1;
                    }
                }
            }

            state_cursor += input_size;
            state_cursor %= WIDTH / 2;
            absorb_rows.push(absorb_row);
        }

        absorb_rows
    }

    pub fn populate_finalize_event<F: PrimeField32>(
        &self,
        finalize_event: &Poseidon2FinalizeEvent<F>,
        num_columns: usize,
    ) -> Vec<F> {
        let mut finalize_row = vec![F::zero(); num_columns];

        {
            let mut cols = self.convert_mut(&mut finalize_row);
            let control_flow = cols.control_flow_mut();
            control_flow.is_finalize = F::one();
            control_flow.is_syscall = F::one();
            control_flow.is_output = F::one();
            control_flow.do_perm = F::from_bool(finalize_event.do_perm);
        }

        {
            let mut cols = self.convert_mut(&mut finalize_row);

            let syscall_params = cols.syscall_params_mut().finalize_mut();
            syscall_params.clk = finalize_event.clk;
            syscall_params.hash_num = finalize_event.hash_num;
            syscall_params.output_ptr = finalize_event.output_ptr;
        }

        {
            let mut cols = self.convert_mut(&mut finalize_row);
            let memory = cols.memory_mut();

            memory.start_addr = finalize_event.output_ptr;
            for i in 0..WIDTH / 2 {
                memory.memory_slot_used[i] = F::one();
                memory.memory_accesses[i].populate(&finalize_event.output_records[i]);
            }
        }

        finalize_row
    }
}

fn populate_external_round<F: PrimeField32>(
    external_rounds_state: &mut [[F; WIDTH]; NUM_EXTERNAL_ROUNDS],
    sbox: &mut Option<&mut [[F; WIDTH]; NUM_EXTERNAL_ROUNDS]>,
    r: usize,
) -> [F; WIDTH] {
    let mut state = {
        let round_state: &mut [F; WIDTH] = external_rounds_state[r].borrow_mut();

        // Add round constants.
        //
        // Optimization: Since adding a constant is a degree 1 operation, we can avoid adding
        // columns for it, and instead include it in the constraint for the x^3 part of the sbox.
        let round = if r < NUM_EXTERNAL_ROUNDS / 2 {
            r
        } else {
            r + NUM_INTERNAL_ROUNDS
        };
        let mut add_rc = *round_state;
        for i in 0..WIDTH {
            add_rc[i] += F::from_wrapped_u32(RC_16_30_U32[round][i]);
        }

        // Apply the sboxes.
        // Optimization: since the linear layer that comes after the sbox is degree 1, we can
        // avoid adding columns for the result of the sbox, and instead include the x^3 -> x^7
        // part of the sbox in the constraint for the linear layer
        let mut sbox_deg_7: [F; 16] = [F::zero(); WIDTH];
        let mut sbox_deg_3: [F; 16] = [F::zero(); WIDTH];
        for i in 0..WIDTH {
            sbox_deg_3[i] = add_rc[i] * add_rc[i] * add_rc[i];
            sbox_deg_7[i] = sbox_deg_3[i] * sbox_deg_3[i] * add_rc[i];
        }

        if let Some(sbox) = sbox.as_deref_mut() {
            sbox[r] = sbox_deg_3;
        }

        sbox_deg_7
    };

    // Apply the linear layer.
    external_linear_layer(&mut state);
    state
}

fn populate_internal_rounds<F: PrimeField32>(
    internal_rounds_state: &mut [F; WIDTH],
    internal_rounds_s0: &mut [F; NUM_INTERNAL_ROUNDS - 1],
    sbox: &mut Option<&mut [F; NUM_INTERNAL_ROUNDS]>,
) -> [F; WIDTH] {
    let mut state: [F; WIDTH] = *internal_rounds_state;
    let mut sbox_deg_3: [F; NUM_INTERNAL_ROUNDS] = [F::zero(); NUM_INTERNAL_ROUNDS];
    for r in 0..NUM_INTERNAL_ROUNDS {
        // Add the round constant to the 0th state element.
        // Optimization: Since adding a constant is a degree 1 operation, we can avoid adding
        // columns for it, just like for external rounds.
        let round = r + NUM_EXTERNAL_ROUNDS / 2;
        let add_rc = state[0] + F::from_wrapped_u32(RC_16_30_U32[round][0]);

        // Apply the sboxes.
        // Optimization: since the linear layer that comes after the sbox is degree 1, we can
        // avoid adding columns for the result of the sbox, just like for external rounds.
        sbox_deg_3[r] = add_rc * add_rc * add_rc;
        let sbox_deg_7 = sbox_deg_3[r] * sbox_deg_3[r] * add_rc;

        // Apply the linear layer.
        state[0] = sbox_deg_7;
        internal_linear_layer(&mut state);

        // Optimization: since we're only applying the sbox to the 0th state element, we only
        // need to have columns for the 0th state element at every step. This is because the
        // linear layer is degree 1, so all state elements at the end can be expressed as a
        // degree-3 polynomial of the state at the beginning of the internal rounds and the 0th
        // state element at rounds prior to the current round
        if r < NUM_INTERNAL_ROUNDS - 1 {
            internal_rounds_s0[r] = state[0];
        }
    }

    let ret_state = state;

    if let Some(sbox) = sbox.as_deref_mut() {
        *sbox = sbox_deg_3;
    }

    ret_state
}
