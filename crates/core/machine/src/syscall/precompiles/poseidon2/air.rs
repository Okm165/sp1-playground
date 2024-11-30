use super::{
    columns::{FullRound, PartialRound, Poseidon2PermuteCols, NUM_POSEIDON2_PERMUTE_COLS},
    Poseidon2PermuteChip,
};
use crate::{air::MemoryAirBuilder, memory::MemoryCols};
use core::borrow::Borrow;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::AbstractField;
use p3_matrix::Matrix;
use sp1_core_executor::syscalls::SyscallCode;
use sp1_primitives::poseidon2::{NUM_FULL_ROUNDS, NUM_PARTIAL_ROUNDS, WIDTH};
use sp1_primitives::{external_linear_layer, internal_linear_layer, RC_16_30_U32};
use sp1_stark::air::{InteractionScope, SP1AirBuilder};

impl<F> BaseAir<F> for Poseidon2PermuteChip {
    fn width(&self) -> usize {
        NUM_POSEIDON2_PERMUTE_COLS
    }
}

impl<AB> Air<AB> for Poseidon2PermuteChip
where
    AB: SP1AirBuilder,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let (local, next) = (main.row_slice(0), main.row_slice(1));
        let local: &Poseidon2PermuteCols<AB::Var> = (*local).borrow();
        let next: &Poseidon2PermuteCols<AB::Var> = (*next).borrow();

        // Constrain the incrementing nonce.
        builder.when_first_row().assert_zero(local.nonce);
        builder.when_transition().assert_eq(local.nonce + AB::Expr::one(), next.nonce);

        // Load from memory to the state
        for (i, word) in local.input_memory.iter().enumerate() {
            builder.assert_eq(local.input_state[i], word.value().reduce::<AB>());
        }

        let mut state: [AB::Expr; WIDTH] = local.input_state.map(|x| x.into());

        // Perform permutation on the state
        external_linear_layer::<AB::Expr>(&mut state);
        builder.assert_all_eq(state, local.state_linear_layer.map(|x| x.into()));

        state = local.state_linear_layer.map(|x| x.into());

        for round in 0..(NUM_FULL_ROUNDS / 2) {
            Self::eval_full_round(
                &state,
                &local.beginning_full_rounds[round],
                &RC_16_30_U32[round].map(AB::F::from_wrapped_u32),
                builder,
            );
            state = local.beginning_full_rounds[round].post.map(|x| x.into());
        }

        for round in 0..NUM_PARTIAL_ROUNDS {
            Self::eval_partial_round(
                &state,
                &local.partial_rounds[round],
                &RC_16_30_U32[round + NUM_FULL_ROUNDS / 2].map(AB::F::from_wrapped_u32)[0],
                builder,
            );
            state = local.partial_rounds[round].post.map(|x| x.into());
        }

        for round in 0..(NUM_FULL_ROUNDS / 2) {
            Self::eval_full_round(
                &state,
                &local.ending_full_rounds[round],
                &RC_16_30_U32[round + NUM_PARTIAL_ROUNDS + NUM_FULL_ROUNDS / 2]
                    .map(AB::F::from_wrapped_u32),
                builder,
            );
            state = local.ending_full_rounds[round].post.map(|x| x.into());
        }

        // Assert that the permuted state is being written to input_memory.
        builder.assert_all_eq(
            state.into_iter().collect::<Vec<AB::Expr>>(),
            local
                .output_memory
                .into_iter()
                .map(|f| f.value().reduce::<AB>())
                .collect::<Vec<AB::Expr>>(),
        );

        // Read input_memory.
        builder.eval_memory_access_slice(
            local.shard,
            local.clk.into(),
            local.input_memory_ptr,
            &local.input_memory,
            local.is_real,
        );

        // Write output_memory.
        builder.eval_memory_access_slice(
            local.shard,
            local.clk.into() + AB::Expr::one(),
            local.output_memory_ptr,
            &local.output_memory,
            local.is_real,
        );

        // Receive the arguments.
        builder.receive_syscall(
            local.shard,
            local.clk,
            local.nonce,
            AB::F::from_canonical_u32(SyscallCode::POSEIDON2_PERMUTE.syscall_id()),
            local.input_memory_ptr,
            local.output_memory_ptr,
            local.is_real,
            InteractionScope::Local,
        );

        // Assert that is_real is a boolean.
        builder.assert_bool(local.is_real);
    }
}

impl Poseidon2PermuteChip {
    pub fn eval_full_round<AB>(
        state: &[AB::Expr; WIDTH],
        full_round: &FullRound<AB::Var>,
        round_constants: &[AB::F; WIDTH],
        builder: &mut AB,
    ) where
        AB: SP1AirBuilder,
    {
        for (i, (s, r)) in state.iter().zip(round_constants.iter()).enumerate() {
            Self::eval_sbox(
                &full_round.sbox_x3[i],
                &full_round.sbox_x7[i],
                &(s.clone() + *r),
                builder,
            );
        }
        let mut committed_sbox_x7 = full_round.sbox_x7.map(|x| x.into());
        external_linear_layer::<AB::Expr>(&mut committed_sbox_x7);
        builder.assert_all_eq(committed_sbox_x7, full_round.post);
    }

    pub fn eval_partial_round<AB>(
        state: &[AB::Expr; WIDTH],
        partial_round: &PartialRound<AB::Var>,
        round_constant: &AB::F,
        builder: &mut AB,
    ) where
        AB: SP1AirBuilder,
    {
        Self::eval_sbox(
            &partial_round.sbox_x3,
            &partial_round.sbox_x7,
            &(state[0].clone() + *round_constant),
            builder,
        );
        let mut committed_state = state.clone();
        committed_state[0] = partial_round.sbox_x7.into();
        internal_linear_layer::<AB::Expr>(&mut committed_state);
        builder.assert_all_eq(committed_state, partial_round.post.map(|x| x.into()));
    }

    #[inline]
    pub fn eval_sbox<AB>(sbox_x3: &AB::Var, sbox_x7: &AB::Var, x: &AB::Expr, builder: &mut AB)
    where
        AB: AirBuilder,
    {
        let committed_x3: AB::Expr = (*sbox_x3).into();
        let committed_x7: AB::Expr = (*sbox_x7).into();
        builder.assert_eq(committed_x7.clone(), committed_x3.square() * x.clone());
    }
}
