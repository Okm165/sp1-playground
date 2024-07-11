use std::borrow::BorrowMut;
use std::mem::size_of;

use p3_air::BaseAir;
use p3_field::PrimeField32;
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use sp1_core::{air::MachineAir, utils::pad_rows_fixed};
use sp1_primitives::RC_16_30_U32;
use tracing::instrument;

use crate::{
    instruction::Instruction::Poseidon2Wide,
    poseidon2_wide::{
        columns::{permutation::max, preprocessed::Poseidon2PreprocessedCols},
        external_linear_layer, internal_linear_layer, Poseidon2WideChip, NUM_EXTERNAL_ROUNDS,
        NUM_INTERNAL_ROUNDS, WIDTH,
    },
    Address, ExecutionRecord, RecursionProgram,
};

const PREPROCESSED_POSEIDON2_WIDTH: usize = size_of::<Poseidon2PreprocessedCols<u8>>();

impl<F: PrimeField32, const DEGREE: usize> MachineAir<F> for Poseidon2WideChip<DEGREE> {
    type Record = ExecutionRecord<F>;

    type Program = RecursionProgram<F>;

    fn name(&self) -> String {
        format!("Poseidon2Wide {}", DEGREE)
    }

    #[instrument(name = "generate poseidon2 wide trace", level = "debug", skip_all, fields(rows = input.poseidon2_wide_events.len()))]
    fn generate_trace(
        &self,
        input: &ExecutionRecord<F>,
        _output: &mut ExecutionRecord<F>,
    ) -> RowMajorMatrix<F> {
        let mut rows = Vec::new();

        let num_columns = <Poseidon2WideChip<DEGREE> as BaseAir<F>>::width(self);

        for event in &input.poseidon2_wide_events {
            let mut row_add = vec![vec![F::zero(); num_columns]; NUM_EXTERNAL_ROUNDS + 2];

            // The first row should have event.input and [event.input[0].clone(); NUM_INTERNAL_ROUNDS-1]
            // in its state columns. The sbox_state will be modified in the computation of the
            // first row.
            {
                let mut cols = self.convert_mut(&mut row_add[0]);
                *cols.get_cols_mut().0 = event.input;
            }

            // For each external round, and once for all the internal rounds at the same time, apply
            // the corresponding operation. This will change the sbox state in row i, and the state
            // and internal_rounds_s0 variable in row i+1.
            for i in 0..NUM_EXTERNAL_ROUNDS + 1 {
                let (next_state_var, next_internal_rounds_s0) = {
                    let mut cols = self.convert_mut(&mut row_add[i]);
                    let (state, internal_state_s0, mut sbox_state) = cols.get_cols_mut();
                    let mut state = *state;
                    if i == 0 {
                        external_linear_layer(&mut state);
                    }
                    let (next_state_var, next_internal_rounds_s0) = if i != NUM_EXTERNAL_ROUNDS / 2
                    {
                        (
                            self.populate_external_round(&state, &mut sbox_state, i),
                            [state[0]; NUM_INTERNAL_ROUNDS - 1],
                        )
                    } else {
                        self.populate_internal_rounds(&state, internal_state_s0, &mut sbox_state)
                    };
                    (next_state_var, next_internal_rounds_s0)
                };
                let mut next_cols = self.convert_mut(&mut row_add[i + 1]);
                let (next_state, internal_state_s0, _) = next_cols.get_cols_mut();
                *next_state = next_state_var;
                *internal_state_s0 = next_internal_rounds_s0;
                if i == NUM_EXTERNAL_ROUNDS {
                    debug_assert_eq!(next_state_var, event.output);
                }
            }
            rows.extend(row_add.into_iter());
        }

        if self.pad {
            // Pad the trace to a power of two.
            // This will need to be adjusted when the AIR constraints are implemented.
            pad_rows_fixed(
                &mut rows,
                || vec![F::zero(); num_columns],
                self.fixed_log2_rows,
            );
        }

        // Convert the trace to a row major matrix.
        let trace =
            RowMajorMatrix::new(rows.into_iter().flatten().collect::<Vec<_>>(), num_columns);

        #[cfg(debug_assertions)]
        println!(
            "poseidon2 wide main trace dims is width: {:?}, height: {:?}",
            trace.width(),
            trace.height()
        );

        trace
    }

    fn included(&self, _record: &Self::Record) -> bool {
        true
    }

    fn preprocessed_width(&self) -> usize {
        PREPROCESSED_POSEIDON2_WIDTH
    }

    /// This is very much subject to change. At the moment, we just populate the memory management
    /// columns. For the moment, we assume that the program only contains Poseidon2Wide instructions,
    /// and for each instruction we read 16 successive memory locations and write to the next 16.
    fn generate_preprocessed_trace(&self, program: &Self::Program) -> Option<RowMajorMatrix<F>> {
        let instruction_count = program
            .instructions
            .iter()
            .filter(|i| matches!(i, Poseidon2Wide(_)))
            .count();

        let mut rows = vec![
            F::zero();
            PREPROCESSED_POSEIDON2_WIDTH
                * max(
                    16,
                    (instruction_count * (NUM_EXTERNAL_ROUNDS + 2)).next_power_of_two()
                )
        ];

        // Set the input memory read management columns
        rows.chunks_mut(PREPROCESSED_POSEIDON2_WIDTH)
            .step_by(NUM_EXTERNAL_ROUNDS + 2)
            .enumerate()
            .for_each(|(event_ct, chunk)| {
                let cols: &mut Poseidon2PreprocessedCols<_> = (*chunk).borrow_mut();
                cols.memory_preprocessed
                    .memory_prepr
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, x)| {
                        x.addr = Address(F::from_canonical_usize(2 * event_ct * WIDTH + i));
                        x.is_real = if event_ct < instruction_count {
                            F::one()
                        } else {
                            F::zero()
                        };
                        x.read_mult = F::one() * x.is_real;
                    })
            });

        // Set the input memory write management columns.
        rows.chunks_mut(PREPROCESSED_POSEIDON2_WIDTH)
            .skip(NUM_EXTERNAL_ROUNDS + 1)
            .step_by(NUM_EXTERNAL_ROUNDS + 2)
            .enumerate()
            .for_each(|(event_ct, chunk)| {
                let cols: &mut Poseidon2PreprocessedCols<_> = (*chunk).borrow_mut();
                cols.memory_preprocessed
                    .memory_prepr
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, x)| {
                        x.addr = Address(F::from_canonical_usize((2 * event_ct + 1) * WIDTH + i));
                        x.is_real = if event_ct < instruction_count {
                            F::one()
                        } else {
                            F::zero()
                        };
                        x.write_mult = F::one() * x.is_real;
                    })
            });

        Some(RowMajorMatrix::new(rows, PREPROCESSED_POSEIDON2_WIDTH))
    }
}

impl<const DEGREE: usize> Poseidon2WideChip<DEGREE> {
    fn populate_external_round<F: PrimeField32>(
        &self,
        round_state: &[F; WIDTH],
        sbox: &mut Option<&mut [F; WIDTH]>,
        r: usize,
    ) -> [F; WIDTH] {
        let mut state = {
            // Add round constants.

            // Optimization: Since adding a constant is a degree 1 operation, we can avoid adding
            // columns for it, and instead include it in the constraint for the x^3 part of the sbox.
            let round = if r < NUM_EXTERNAL_ROUNDS / 2 {
                r
            } else {
                r + NUM_INTERNAL_ROUNDS - 1
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
                *sbox = sbox_deg_3;
            }

            sbox_deg_7
        };

        // Apply the linear layer.
        external_linear_layer(&mut state);
        state
    }

    fn populate_internal_rounds<F: PrimeField32>(
        &self,
        state: &[F; WIDTH],
        internal_rounds_s0: &mut [F; NUM_INTERNAL_ROUNDS - 1],
        sbox: &mut Option<&mut [F; WIDTH]>,
    ) -> ([F; WIDTH], [F; NUM_INTERNAL_ROUNDS - 1]) {
        let mut new_state = *state;
        let mut sbox_deg_3: [F; max(WIDTH, NUM_INTERNAL_ROUNDS)] =
            [F::zero(); max(WIDTH, NUM_INTERNAL_ROUNDS)];
        for r in 0..NUM_INTERNAL_ROUNDS {
            // Add the round constant to the 0th state element.
            // Optimization: Since adding a constant is a degree 1 operation, we can avoid adding
            // columns for it, just like for external rounds.
            let round = r + NUM_EXTERNAL_ROUNDS / 2;
            let add_rc = new_state[0] + F::from_wrapped_u32(RC_16_30_U32[round][0]);

            // Apply the sboxes.
            // Optimization: since the linear layer that comes after the sbox is degree 1, we can
            // avoid adding columns for the result of the sbox, just like for external rounds.
            sbox_deg_3[r] = add_rc * add_rc * add_rc;
            let sbox_deg_7 = sbox_deg_3[r] * sbox_deg_3[r] * add_rc;

            // Apply the linear layer.
            new_state[0] = sbox_deg_7;
            internal_linear_layer(&mut new_state);

            // Optimization: since we're only applying the sbox to the 0th state element, we only
            // need to have columns for the 0th state element at every step. This is because the
            // linear layer is degree 1, so all state elements at the end can be expressed as a
            // degree-3 polynomial of the state at the beginning of the internal rounds and the 0th
            // state element at rounds prior to the current round
            if r < NUM_INTERNAL_ROUNDS - 1 {
                internal_rounds_s0[r] = new_state[0];
            }
        }

        let ret_state = new_state;

        if let Some(sbox) = sbox.as_deref_mut() {
            *sbox = sbox_deg_3;
        }

        (ret_state, *internal_rounds_s0)
    }
}

#[cfg(test)]
mod tests {
    use p3_baby_bear::BabyBear;
    use p3_field::AbstractField;
    use p3_matrix::dense::RowMajorMatrix;
    use p3_symmetric::Permutation;
    use sp1_core::air::MachineAir;
    use sp1_core::utils::inner_perm;
    use zkhash::ark_ff::UniformRand;

    use crate::{
        poseidon2_wide::{Poseidon2WideChip, WIDTH},
        ExecutionRecord, Poseidon2WideEvent,
    };

    #[test]
    fn generate_trace_deg_3() {
        type F = BabyBear;
        let input_0 = [F::one(); WIDTH];
        let permuter = inner_perm();
        let output_0 = permuter.permute(input_0);
        let mut rng = rand::thread_rng();

        let input_1 = [F::rand(&mut rng); WIDTH];
        let output_1 = permuter.permute(input_1);

        let shard = ExecutionRecord {
            poseidon2_wide_events: vec![
                Poseidon2WideEvent {
                    input: input_0,
                    output: output_0,
                },
                Poseidon2WideEvent {
                    input: input_1,
                    output: output_1,
                },
            ],
            ..Default::default()
        };
        let chip_3 = Poseidon2WideChip::<3>::default();
        let _: RowMajorMatrix<F> = chip_3.generate_trace(&shard, &mut ExecutionRecord::default());
    }

    #[test]
    fn generate_trace_deg_9() {
        type F = BabyBear;
        let input_0 = [F::one(); WIDTH];
        let permuter = inner_perm();
        let output_0 = permuter.permute(input_0);
        let mut rng = rand::thread_rng();

        let input_1 = [F::rand(&mut rng); WIDTH];
        let output_1 = permuter.permute(input_1);

        let shard = ExecutionRecord {
            poseidon2_wide_events: vec![
                Poseidon2WideEvent {
                    input: input_0,
                    output: output_0,
                },
                Poseidon2WideEvent {
                    input: input_1,
                    output: output_1,
                },
            ],
            ..Default::default()
        };
        let chip_9 = Poseidon2WideChip::<9>::default();
        let _: RowMajorMatrix<F> = chip_9.generate_trace(&shard, &mut ExecutionRecord::default());
    }
}
