use core::borrow::Borrow;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_baby_bear::MONTY_INVERSE;
use p3_baby_bear::POSEIDON2_INTERNAL_MATRIX_DIAG_16_BABYBEAR_MONTY;
use p3_field::AbstractField;
use p3_field::PrimeField32;
use p3_matrix::Matrix;
use sp1_primitives::RC_16_30_U32;
use sp1_stark::air::SP1AirBuilder;

use super::{
    columns::{FullRound, PartialRound, Poseidon2PermCols, NUM_POSEIDON2PERM_COLS},
    Poseidon2PermChip, NUM_FULL_ROUNDS, NUM_PARTIAL_ROUNDS, WIDTH,
};

impl<F> BaseAir<F> for Poseidon2PermChip {
    fn width(&self) -> usize {
        NUM_POSEIDON2PERM_COLS
    }
}

impl<AB> Air<AB> for Poseidon2PermChip
where
    AB: SP1AirBuilder,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let (local, next) = (main.row_slice(0), main.row_slice(1));
        let local: &Poseidon2PermCols<AB::Var> = (*local).borrow();
        let next: &Poseidon2PermCols<AB::Var> = (*next).borrow();

        // Constrain the incrementing nonce.
        builder.when_first_row().assert_zero(local.nonce);
        builder.when_transition().assert_eq(local.nonce + AB::Expr::one(), next.nonce);

        let mut state: [AB::Expr; WIDTH] = local.state.map(|x| x.into());

        Self::external_linear_layer::<AB>(&mut state);

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
    }
}

impl Poseidon2PermChip {
    pub fn external_linear_layer<AB: SP1AirBuilder>(state: &mut [AB::Expr; WIDTH]) {
        for j in (0..WIDTH).step_by(4) {
            Self::apply_m_4::<AB>(&mut state[j..j + 4]);
        }
        let sums: [AB::Expr; 4] = core::array::from_fn(|k| {
            (0..WIDTH).step_by(4).map(|j| state[j + k].clone()).sum::<AB::Expr>()
        });

        for j in 0..WIDTH {
            state[j] = state[j].clone() + sums[j % 4].clone();
        }
    }

    pub fn internal_linear_layer<AB: SP1AirBuilder>(state: &mut [AB::Expr; WIDTH]) {
        let matmul_constants: [AB::F; WIDTH] = POSEIDON2_INTERNAL_MATRIX_DIAG_16_BABYBEAR_MONTY
            .iter()
            .map(|x| AB::F::from_wrapped_u32(x.as_canonical_u32()))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        Self::matmul_internal::<AB>(state, matmul_constants);
        let monty_inverse = AB::F::from_wrapped_u32(MONTY_INVERSE.as_canonical_u32());
        state.iter_mut().for_each(|i| *i = i.clone() * monty_inverse);
    }

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
        Self::external_linear_layer::<AB>(state);
        for (state_i, post_i) in state.iter_mut().zip(full_round.post) {
            builder.assert_eq(state_i.clone(), post_i);
            *state_i = post_i.into();
        }
    }

    fn eval_partial_round<AB>(
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

        Self::internal_linear_layer::<AB>(state);
    }

    pub fn apply_m_4<AB>(x: &mut [AB::Expr])
    where
        AB: SP1AirBuilder,
    {
        let t01 = x[0].clone() + x[1].clone();
        let t23 = x[2].clone() + x[3].clone();
        let t0123 = t01.clone() + t23.clone();
        let t01123 = t0123.clone() + x[1].clone();
        let t01233 = t0123.clone() + x[3].clone();
        // The order here is important. Need to overwrite x[0] and x[2] after x[1] and x[3].
        x[3] = t01233.clone() + x[0].double(); // 3*x[0] + x[1] + x[2] + 2*x[3]
        x[1] = t01123.clone() + x[2].double(); // x[0] + 2*x[1] + 3*x[2] + x[3]
        x[0] = t01123 + t01; // 2*x[0] + 3*x[1] + x[2] + x[3]
        x[2] = t01233 + t23; // x[0] + x[1] + 2*x[2] + 3*x[3]
    }

    pub fn matmul_internal<AB: SP1AirBuilder>(
        state: &mut [AB::Expr; WIDTH],
        mat_internal_diag_m_1: [AB::F; WIDTH],
    ) {
        let sum: AB::Expr = state.iter().cloned().sum();
        for i in 0..WIDTH {
            state[i] = state[i].clone() * mat_internal_diag_m_1[i];
            state[i] = state[i].clone() + sum.clone();
        }
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
