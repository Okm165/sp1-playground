mod air;
mod columns;
mod trace;

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
