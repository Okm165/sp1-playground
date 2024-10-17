use sp1_derive::AlignedBorrow;
use sp1_stark::Word;
use std::mem::size_of;

use crate::{
    memory::MemoryReadWriteCols,
    operations::{BabyBearWordRangeChecker, IsEqualWordOperation, IsZeroOperation},
};

use super::MaximalByteCol;

pub const NUM_MEMORY_COLUMNS: usize = size_of::<MemoryColumns<u8>>();

/// The column layout for memory.
#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryColumns<T> {
    // An addr that we are reading from or writing to as a word. We are guaranteed that this does
    // not overflow the field when reduced.

    // The relationships among addr_word, addr_aligned, and addr_offset is as follows:
    // addr_aligned = addr_word - addr_offset
    // addr_offset = addr_word % 4
    // Note that this all needs to be verified in the AIR
    pub addr_word: Word<T>,

    /// A flag indicating whether the most significant byte of the address less than 120.
    pub addr_word_range_checker: T,

    pub addr_aligned: T,
    /// The LE bit decomp of the least significant byte of address aligned.
    pub aa_least_sig_byte_decomp: [T; 6],
    pub addr_offset: T,
    pub memory_access: MemoryReadWriteCols<T>,

    pub offset_is_one: T,
    pub offset_is_two: T,
    pub offset_is_three: T,

    // LE bit decomposition for the most significant byte of memory value.  This is used to
    // determine the sign for that value (used for LB and LH).
    pub most_sig_byte_decomp: [T; 8],

    pub addr_word_nonce: T,
    pub unsigned_mem_val_nonce: T,
}

impl<T: Copy> MaximalByteCol<T> for MemoryColumns<T> {
    fn most_significant_byte(&self) -> T {
        self.addr_word[3]
    }
}
