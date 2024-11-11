use super::WIDTH;
use crate::{
    events::{Poseidon2PermuteEvent, PrecompileEvent},
    syscalls::{
        precompiles::poseidon2::{NUM_FULL_ROUNDS, NUM_PARTIAL_ROUNDS},
        Syscall, SyscallCode, SyscallContext,
    },
};
use p3_baby_bear::{BabyBear, MONTY_INVERSE, POSEIDON2_INTERNAL_MATRIX_DIAG_16_BABYBEAR_MONTY};
use p3_field::{AbstractField, PrimeField32};
use sp1_primitives::RC_16_30_U32;

type F = BabyBear;

pub(crate) struct Poseidon2PermuteSyscall;

impl Poseidon2PermuteSyscall {
    pub fn full_round<F: PrimeField32>(state: &mut [F; WIDTH], round_constants: &[F; WIDTH]) {
        for (s, r) in state.iter_mut().zip(round_constants.iter()) {
            *s += *r;
            Self::sbox(s);
        }

        external_linear_layer(state);
    }

    pub fn partial_round<F: PrimeField32>(state: &mut [F; WIDTH], round_constant: &F) {
        state[0] += *round_constant;
        Self::sbox(&mut state[0]);

        internal_linear_layer(state);
    }

    #[inline]
    pub fn sbox<F: PrimeField32>(x: &mut F) {
        *x = x.exp_const_u64::<7>();
    }
}

impl Syscall for Poseidon2PermuteSyscall {
    fn num_extra_cycles(&self) -> u32 {
        1
    }

    fn execute(
        &self,
        rt: &mut SyscallContext,
        syscall_code: SyscallCode,
        arg1: u32,
        arg2: u32,
    ) -> Option<u32> {
        let clk_init = rt.clk;
        let input_ptr = arg1;
        assert!(arg2 == 0, "arg2 must be 0");

        let input_ptr_init = input_ptr;

        let (_, input_memory_values) = rt.mr_slice(input_ptr, WIDTH);
        let mut state: [F; WIDTH] = input_memory_values
            .into_iter()
            .map(F::from_canonical_u32)
            .collect::<Vec<F>>()
            .try_into()
            .unwrap();

        // Perform permutation on the state
        external_linear_layer(&mut state);

        for round in 0..NUM_FULL_ROUNDS / 2 {
            Self::full_round(&mut state, &RC_16_30_U32[round].map(F::from_canonical_u32));
        }

        for round in 0..NUM_PARTIAL_ROUNDS {
            Self::partial_round(&mut state, &RC_16_30_U32[round].map(F::from_canonical_u32)[0]);
        }

        for round in 0..NUM_FULL_ROUNDS / 2 {
            Self::full_round(&mut state, &RC_16_30_U32[round].map(F::from_canonical_u32));
        }

        let input_memory_records =
            rt.mw_slice(input_ptr, state.map(|f| f.as_canonical_u32()).as_slice());

        // Push the SHA extend event.
        let lookup_id = rt.syscall_lookup_id;
        let shard = rt.current_shard();
        let event = PrecompileEvent::Poseidon2Permute(Poseidon2PermuteEvent {
            lookup_id,
            shard,
            clk: clk_init,
            input_ptr: input_ptr_init,
            input_memory_records,
            local_mem_access: rt.postprocess(),
        });
        let syscall_event =
            rt.rt.syscall_event(clk_init, syscall_code.syscall_id(), arg1, arg2, lookup_id);
        rt.add_precompile_event(syscall_code, syscall_event, event);

        None
    }
}

pub fn external_linear_layer<F: AbstractField>(state: &mut [F; WIDTH]) {
    for j in (0..WIDTH).step_by(4) {
        apply_m_4::<F>(&mut state[j..j + 4]);
    }
    let sums: [F; 4] =
        core::array::from_fn(|k| (0..WIDTH).step_by(4).map(|j| state[j + k].clone()).sum::<F>());

    for j in 0..WIDTH {
        state[j] = state[j].clone() + sums[j % 4].clone();
    }
}

pub fn internal_linear_layer<F: AbstractField + Clone>(state: &mut [F; WIDTH]) {
    let matmul_constants: [F; WIDTH] = POSEIDON2_INTERNAL_MATRIX_DIAG_16_BABYBEAR_MONTY
        .iter()
        .map(|x| F::from_wrapped_u32(x.as_canonical_u32()))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    matmul_internal(state, matmul_constants);
    let monty_inverse = F::from_wrapped_u32(MONTY_INVERSE.as_canonical_u32());
    state.iter_mut().for_each(|i| *i = i.clone() * monty_inverse.clone());
}

pub fn apply_m_4<F: AbstractField>(x: &mut [F]) {
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

pub fn matmul_internal<F: AbstractField + Clone>(
    state: &mut [F; WIDTH],
    mat_internal_diag_m_1: [F; WIDTH],
) {
    let sum: F = state.iter().cloned().sum();
    for (state, mat_internal) in state.iter_mut().zip(mat_internal_diag_m_1) {
        *state = (state.clone() * mat_internal) + sum.clone();
    }
}
