use super::{NUM_FULL_ROUNDS, NUM_PARTIAL_ROUNDS, WIDTH};
use sp1_derive::AlignedBorrow;
pub const NUM_POSEIDON2PERM_COLS: usize = size_of::<Poseidon2PermCols<u8>>();

/// A set of columns needed to compute the Poseidon2Permutation function.
///
#[derive(AlignedBorrow)]
#[repr(C)]
pub struct Poseidon2PermCols<T> {
    /// Inputs.
    pub shard: T,
    pub nonce: T,
    pub clk: T,
    pub inputs: [T; WIDTH],

    /// Beginning Full Rounds
    pub beginning_full_rounds: [FullRound<T>; NUM_FULL_ROUNDS],

    /// Partial Rounds
    pub partial_rounds: [PartialRound<T>; NUM_PARTIAL_ROUNDS],

    /// Ending Full Rounds
    pub ending_full_rounds: [FullRound<T>; NUM_FULL_ROUNDS],
}

/// Full round columns.
#[repr(C)]
pub struct FullRound<T> {
    pub sbox: [T; WIDTH],
    pub post: [T; WIDTH],
}

/// Partial round columns.
#[repr(C)]
pub struct PartialRound<T> {
    pub sbox: [T; WIDTH],
    pub post_sbox: T,
}
