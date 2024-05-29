use sp1_derive::AlignedBorrow;
use std::mem::size_of;

use crate::{air::Word, operations::BabyBearRangeChecker};

pub const NUM_JUMP_COLS: usize = size_of::<JumpCols<u8>>();

#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct JumpCols<T> {
    /// The current program counter.
    pub pc: Word<T>,
    pub pc_range_checker: BabyBearRangeChecker<T>,

    /// THe next program counter.
    pub next_pc: Word<T>,
    pub next_pc_range_checker: BabyBearRangeChecker<T>,
}
