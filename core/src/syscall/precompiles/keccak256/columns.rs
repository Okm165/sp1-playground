use core::borrow::{Borrow, BorrowMut};
use core::mem::size_of;

use sp1_derive::AlignedBorrow;

use crate::memory::MemoryReadWriteCols;

use super::STATE_NUM_WORDS;

#[derive(AlignedBorrow)]
#[repr(C)]
pub(crate) struct KeccakMemCols<T> {
    pub shard: T,
    pub clk: T,

    // pub p3_keccak_cols: P3KeccakCols<T>,
    pub state_mem: [MemoryReadWriteCols<T>; STATE_NUM_WORDS],
    pub state_addr: T,

    pub do_memory_check: T,

    pub is_real: T,
}

pub const NUM_KECCAK_MEM_COLS: usize = size_of::<KeccakMemCols<u8>>();
// pub const P3_KECCAK_COLS_OFFSET: usize = offset_of!(KeccakCols<u8>, p3_keccak_cols);
