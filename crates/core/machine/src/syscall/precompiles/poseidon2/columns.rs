use crate::memory::{MemoryReadCols, MemoryWriteCols};
use sp1_derive::AlignedBorrow;
use sp1_primitives::poseidon2::{NUM_FULL_ROUNDS, NUM_PARTIAL_ROUNDS, WIDTH};
pub const NUM_POSEIDON2_PERMUTE_COLS: usize = size_of::<Poseidon2PermuteCols<u8>>();

/// A set of columns needed to compute the Poseidon2 permutation function.
///
#[derive(AlignedBorrow)]
#[repr(C)]
pub struct Poseidon2PermuteCols<T> {
    /// Inputs.
    pub shard: T,
    pub nonce: T,
    pub clk: T,

    pub input_memory_ptr: T,
    pub input_memory: [MemoryReadCols<T>; WIDTH],

    pub output_memory_ptr: T,
    pub output_memory: [MemoryWriteCols<T>; WIDTH],

    pub input_state: [T; WIDTH],
    pub state_linear_layer: [T; WIDTH],

    /// Beginning Full Rounds
    pub beginning_full_rounds: [FullRound<T>; NUM_FULL_ROUNDS / 2],

    /// Partial Rounds
    pub partial_rounds: [PartialRound<T>; NUM_PARTIAL_ROUNDS],

    /// Ending Full Rounds
    pub ending_full_rounds: [FullRound<T>; NUM_FULL_ROUNDS / 2],

    pub is_real: T,
}

/// Full round columns.
#[repr(C)]
pub struct FullRound<T> {
    pub sbox_x3: [T; WIDTH],
    pub sbox_x7: [T; WIDTH],
    pub post: [T; WIDTH],
}

/// Partial round columns.
#[repr(C)]
pub struct PartialRound<T> {
    pub sbox_x3: T,
    pub sbox_x7: T,
    pub post: [T; WIDTH],
}
