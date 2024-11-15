use crate::{
    events::{Poseidon2PermuteEvent, PrecompileEvent},
    syscalls::{Syscall, SyscallCode, SyscallContext},
};
use p3_baby_bear::BabyBear;
use p3_field::{AbstractField, PrimeField32};
use sp1_primitives::{
    external_linear_layer, internal_linear_layer, NUM_FULL_ROUNDS, NUM_PARTIAL_ROUNDS,
    RC_16_30_U32, WIDTH,
};

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
            .map(F::from_wrapped_u32)
            .collect::<Vec<F>>()
            .try_into()
            .unwrap();

        // Perform permutation on the state
        external_linear_layer(&mut state);

        for round in 0..NUM_FULL_ROUNDS / 2 {
            Self::full_round(&mut state, &RC_16_30_U32[round].map(F::from_wrapped_u32));
        }

        for round in 0..NUM_PARTIAL_ROUNDS {
            Self::partial_round(&mut state, &RC_16_30_U32[round].map(F::from_wrapped_u32)[0]);
        }

        for round in 0..NUM_FULL_ROUNDS / 2 {
            Self::full_round(&mut state, &RC_16_30_U32[round].map(F::from_wrapped_u32));
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
