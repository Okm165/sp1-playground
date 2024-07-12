use std::borrow::BorrowMut;
use std::mem::size_of;

use itertools::Itertools;
use p3_air::BaseAir;
use p3_field::PrimeField32;
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use sp1_core::{air::MachineAir, utils::pad_rows_fixed};
use sp1_primitives::RC_16_30_U32;
use tracing::instrument;

use crate::{
    instruction::Instruction::Poseidon2Skinny,
    mem::MemoryPreprocessedCols,
    poseidon2_wide::{
        columns::{permutation::permutation_mut, preprocessed::Poseidon2PreprocessedCols},
        external_linear_layer, internal_linear_layer, Poseidon2WideChip, NUM_EXTERNAL_ROUNDS,
        NUM_INTERNAL_ROUNDS, WIDTH,
    },
    ExecutionRecord, RecursionProgram,
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
            let mut input_row = vec![F::zero(); num_columns];
            let mut permutation = permutation_mut::<F, DEGREE>(&mut input_row);

            let (
                external_rounds_state,
                internal_rounds_state,
                internal_rounds_s0,
                mut external_sbox,
                mut internal_sbox,
                output_state,
            ) = permutation.get_cols_mut();

            external_rounds_state[0] = event.input;
            external_linear_layer(&mut external_rounds_state[0]);

            // Apply the first half of external rounds.
            for r in 0..NUM_EXTERNAL_ROUNDS / 2 {
                let next_state =
                    self.populate_external_round(external_rounds_state, &mut external_sbox, r);
                if r == NUM_EXTERNAL_ROUNDS / 2 - 1 {
                    *internal_rounds_state = next_state;
                } else {
                    external_rounds_state[r + 1] = next_state;
                }
            }

            // Apply the internal rounds.
            external_rounds_state[NUM_EXTERNAL_ROUNDS / 2] = self.populate_internal_rounds(
                internal_rounds_state,
                internal_rounds_s0,
                &mut internal_sbox,
            );

            // Apply the second half of external rounds.
            for r in NUM_EXTERNAL_ROUNDS / 2..NUM_EXTERNAL_ROUNDS {
                let next_state =
                    self.populate_external_round(external_rounds_state, &mut external_sbox, r);
                if r == NUM_EXTERNAL_ROUNDS - 1 {
                    for i in 0..WIDTH {
                        output_state[i] = next_state[i];
                        assert_eq!(event.output[i], next_state[i]);
                    }
                } else {
                    external_rounds_state[r + 1] = next_state;
                }
            }
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

    fn generate_preprocessed_trace(&self, program: &Self::Program) -> Option<RowMajorMatrix<F>> {
        let instructions =
            program
                .instructions
                .iter()
                .filter_map(|instruction| match instruction {
                    Poseidon2Skinny(instr) => Some(instr),
                    _ => None,
                });

        let num_instructions = instructions.clone().count();

        let mut rows = vec![[F::zero(); PREPROCESSED_POSEIDON2_WIDTH]; num_instructions];

        // Iterate over the instructions and take NUM_EXTERNAL_ROUNDS + 2 rows for each instruction.
        instructions
            .zip_eq(rows.iter_mut())
            .for_each(|(instruction, row)| {
                let cols: &mut Poseidon2PreprocessedCols<_> = (*row).as_mut_slice().borrow_mut();

                // Set the memory columns. We read once, at the first iteration,
                // and write once, at the last iteration.
                cols.memory_preprocessed = std::array::from_fn(|j| {
                    if j < 16 {
                        MemoryPreprocessedCols {
                            addr: instruction.addrs.input[j],
                            read_mult: F::one(),
                            write_mult: F::zero(),
                            is_real: F::one(),
                        }
                    } else {
                        MemoryPreprocessedCols {
                            addr: instruction.addrs.output[j],
                            read_mult: F::zero(),
                            write_mult: instruction.mults[j],
                            is_real: F::one(),
                        }
                    }
                });
            });
        if self.pad {
            // Pad the trace to a power of two.
            // This may need to be adjusted when the AIR constraints are implemented.
            pad_rows_fixed(
                &mut rows,
                || [F::zero(); PREPROCESSED_POSEIDON2_WIDTH],
                self.fixed_log2_rows,
            );
        }
        let trace_rows = rows.into_iter().flatten().collect::<Vec<_>>();
        Some(RowMajorMatrix::new(
            trace_rows,
            PREPROCESSED_POSEIDON2_WIDTH,
        ))
    }
}

impl<const DEGREE: usize> Poseidon2WideChip<DEGREE> {
    fn populate_external_round<F: PrimeField32>(
        &self,
        external_rounds_state: &[[F; WIDTH]],
        sbox: &mut Option<&mut [[F; WIDTH]; NUM_EXTERNAL_ROUNDS]>,
        r: usize,
    ) -> [F; WIDTH] {
        let mut state = {
            let round_state: &[F; WIDTH] = &external_rounds_state[r];

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
        &self,
        internal_rounds_state: &[F; WIDTH],
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
        ExecutionRecord, Poseidon2SkinnyEvent,
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
                Poseidon2SkinnyEvent {
                    input: input_0,
                    output: output_0,
                },
                Poseidon2SkinnyEvent {
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
                Poseidon2SkinnyEvent {
                    input: input_0,
                    output: output_0,
                },
                Poseidon2SkinnyEvent {
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
