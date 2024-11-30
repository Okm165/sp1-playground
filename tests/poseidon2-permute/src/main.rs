#![no_main]
sp1_zkvm::entrypoint!(main);
use p3_baby_bear::BabyBear;
use p3_field::{self, AbstractField, PrimeField32};
use p3_symmetric::Permutation;
use sp1_primitives::poseidon2_init;
use sp1_zkvm::syscalls::syscall_poseidon2_permute;

pub fn main() {
    let input: [u32; 16] = (0..16).collect::<Vec<u32>>().try_into().unwrap();
    let mut state_precompile = input.clone();
    let mut state_native = input.clone();
    for _ in 0..20 {
        let mut out: [u32; 16] = [0; 16];
        syscall_poseidon2_permute(&state_precompile, &mut out);
        state_precompile = out;
    }

    let poseidon = poseidon2_init();
    for _ in 0..20 {
        state_native = poseidon
            .permute(state_native.map(BabyBear::from_canonical_u32))
            .map(|f| f.as_canonical_u32());
    }

    assert_eq!(state_precompile, state_native);
}
