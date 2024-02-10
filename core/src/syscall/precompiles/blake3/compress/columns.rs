use core::borrow::Borrow;
use core::borrow::BorrowMut;
use std::mem::size_of;

use valida_derive::AlignedBorrow;

use crate::memory::MemoryReadCols;
use crate::memory::MemoryReadWriteCols;
use crate::memory::MemoryWriteCols;

use super::g::GOperation;
use super::G_INPUT_SIZE;
use super::G_OUTPUT_SIZE;
use super::NUM_MSG_WORDS_PER_CALL;
use super::NUM_STATE_WORDS_PER_CALL;
use super::OPERATION_COUNT;
use super::ROUND_COUNT;

pub const NUM_BLAKE3_COMPRESS_INNER_COLS: usize = size_of::<Blake3CompressInnerCols<u8>>();

/// Cols to perform the Compress
#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct Blake3CompressInnerCols<T> {
    pub segment: T,
    pub clk: T,

    pub state_ptr: T,

    pub message_ptr: T,

    /// Reads and writes a part of the state.
    pub state_reads_writes: [MemoryReadWriteCols<T>; NUM_STATE_WORDS_PER_CALL],

    /// Reads a part of the message.
    pub message_reads: [MemoryReadCols<T>; NUM_MSG_WORDS_PER_CALL],

    /// Indicates which call of `g` is being performed.
    pub operation_index: T,

    pub is_operation_index_n: [T; OPERATION_COUNT],

    /// Indicates which call of `round` is being performed.
    pub round_index: T,

    pub is_round_index_n: [T; ROUND_COUNT],

    /// The indices to pass to `g`.
    pub state_index: [T; NUM_STATE_WORDS_PER_CALL],

    /// The two values from `MSG_SCHEDULE` to pass to `g`.
    /// TODO: I don't think I need this.
    pub msg_schedule: [T; NUM_MSG_WORDS_PER_CALL],

    pub g: GOperation<T>,

    pub is_real: T,
}
