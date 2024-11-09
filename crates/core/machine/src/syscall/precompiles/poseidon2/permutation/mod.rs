mod air;
mod columns;
mod trace;

/// The width of the permutation.
pub const WIDTH: usize = 16;
pub const RATE: usize = WIDTH / 2;

pub const NUM_FULL_ROUNDS: usize = 8;
pub const NUM_PARTIAL_ROUNDS: usize = 13;
pub const NUM_ROUNDS: usize = NUM_FULL_ROUNDS + NUM_PARTIAL_ROUNDS;

/// Implements the Poseidon2 permutation operation.
#[derive(Default)]
pub struct Poseidon2PermuteChip;

impl Poseidon2PermuteChip {
    pub const fn new() -> Self {
        Self {}
    }
}

#[cfg(test)]
pub mod hash_tests {}
