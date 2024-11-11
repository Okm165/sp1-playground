use crate::air::MemoryAirBuilder;
use crate::memory::{value_as_limbs, MemoryCols};
use crate::operations::BabyBearWordRangeChecker;
use core::borrow::Borrow;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::AbstractField;
use p3_matrix::Matrix;
use sp1_core_executor::syscalls::precompiles::poseidon2::{
    self, NUM_FULL_ROUNDS, NUM_PARTIAL_ROUNDS, WIDTH,
};
use sp1_core_executor::syscalls::SyscallCode;
use sp1_primitives::RC_16_30_U32;
use sp1_stark::air::{BaseAirBuilder, InteractionScope, SP1AirBuilder};

use super::{
    columns::{FullRound, PartialRound, Poseidon2PermuteCols, NUM_POSEIDON2_PERMUTE_COLS},
    Poseidon2PermuteChip,
};

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
            BabyBearWordRangeChecker::<AB::F>::range_check(
                builder,
                *word.prev_value(),
                local.input_range_checker[i],
                local.is_real.into(),
            );
            builder.assert_eq(local.state[i], word.prev_value().reduce::<AB>());
        }

        let mut state: [AB::Expr; WIDTH] = local.state.map(|x| x.into());

        // Perform permutation on the state
        poseidon2::permutation::external_linear_layer::<AB::Expr>(&mut state);

        for round in 0..NUM_FULL_ROUNDS / 2 {
            Self::eval_full_round(
                &mut state,
                &local.beginning_full_rounds[round],
                &RC_16_30_U32[round].map(AB::F::from_canonical_u32),
                builder,
            );
        }

        for round in 0..NUM_PARTIAL_ROUNDS {
            Self::eval_partial_round(
                &mut state,
                &local.partial_rounds[round],
                &RC_16_30_U32[round].map(AB::F::from_canonical_u32)[0],
                builder,
            );
        }

        for round in 0..NUM_FULL_ROUNDS / 2 {
            Self::eval_full_round(
                &mut state,
                &local.ending_full_rounds[round],
                &RC_16_30_U32[round].map(AB::F::from_canonical_u32),
                builder,
            );
        }

        // Assert that the permuted state is being written to input_memory.
        builder.when(local.is_real).assert_all_eq(local.state, value_as_limbs(&local.input_memory));

        // Read and write input_memory.
        builder.eval_memory_access_slice(
            local.shard,
            local.clk.into() + AB::Expr::one(),
            local.input_ptr,
            &local.input_memory,
            local.is_real,
        );

        // Receive the arguments.
        builder.receive_syscall(
            local.shard,
            local.clk,
            local.nonce,
            AB::F::from_canonical_u32(SyscallCode::POSEIDON2_PERMUTE.syscall_id()),
            local.input_ptr,
            AB::Expr::zero(),
            local.is_real,
            InteractionScope::Local,
        );

        // Assert that is_real is a boolean.
        builder.assert_bool(local.is_real);
    }
}

impl Poseidon2PermuteChip {
    pub fn eval_full_round<AB>(
        state: &mut [AB::Expr; WIDTH],
        full_round: &FullRound<AB::Var>,
        round_constants: &[AB::F; WIDTH],
        builder: &mut AB,
    ) where
        AB: SP1AirBuilder,
    {
        for (i, (s, r)) in state.iter_mut().zip(round_constants.iter()).enumerate() {
            *s = s.clone() + *r;
            Self::eval_sbox(&full_round.sbox[i], s, builder);
        }
        poseidon2::permutation::external_linear_layer::<AB::Expr>(state);
        for (state_i, post_i) in state.iter_mut().zip(full_round.post) {
            builder.assert_eq(state_i.clone(), post_i);
            *state_i = post_i.into();
        }
    }

    pub fn eval_partial_round<AB>(
        state: &mut [AB::Expr; WIDTH],
        partial_round: &PartialRound<AB::Var>,
        round_constant: &AB::F,
        builder: &mut AB,
    ) where
        AB: SP1AirBuilder,
    {
        state[0] = state[0].clone() + *round_constant;
        Self::eval_sbox(&partial_round.sbox[0], &mut state[0], builder);

        builder.assert_eq(state[0].clone(), partial_round.post_sbox);
        state[0] = partial_round.post_sbox.into();

        poseidon2::permutation::internal_linear_layer::<AB::Expr>(state);
    }

    #[inline]
    pub fn eval_sbox<AB>(sbox: &AB::Var, x: &mut AB::Expr, builder: &mut AB)
    where
        AB: AirBuilder,
    {
        *x = x.exp_const_u64::<7>();
        builder.assert_eq(*sbox, x.clone());
    }
}
