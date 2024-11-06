use sp1_derive::AlignedBorrow;
use std::mem::size_of;

use super::NUM_BYTE3_OPS;

/// The number of main trace columns for `Byte3Chip`.
pub const NUM_BYTE3_PREPROCESSED_COLS: usize = size_of::<Byte3PreprocessedCols<u8>>();

/// The number of multiplicity columns for `Byte3Chip`.
pub const NUM_BYTE3_MULT_COLS: usize = size_of::<Byte3MultCols<u8>>();

#[derive(Debug, Clone, Copy, AlignedBorrow)]
#[repr(C)]
pub struct Byte3PreprocessedCols<T> {
    /// The first byte operand.
    pub a: T,

    /// The second byte operand.
    pub b: T,

    /// The third byte operand.
    pub c: [T; 256],

    /// The result of the `a xor b xor c`
    pub xor3: [T; 256],

    /// The result of the `ch := (e and f) xor ((not e) and g)`.
    pub ch: [T; 256],

    /// The result of the `maj := (a and b) xor (a and c) xor (b and c)`.
    pub maj: [T; 256],
}

/// For each byte operation in the preprocessed table, a corresponding Byte3MultCols row tracks the
/// number of times the operation is used.
#[derive(Debug, Clone, Copy, AlignedBorrow)]
#[repr(C)]
pub struct Byte3MultCols<T> {
    /// The multiplicities of each byte operation.
    pub multiplicities: [[T; 256]; NUM_BYTE3_OPS],
}
