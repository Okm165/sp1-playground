use crate::{air::Word, memory::MemoryReadWriteCols};
use sp1_core::operations::IsZeroOperation;
use sp1_derive::AlignedBorrow;

/// The column layout for the chip.
#[derive(AlignedBorrow, Default, Clone, Copy, Debug)]
#[repr(C)]
pub struct CpuCols<T> {
    pub clk: T,
    pub pc: T,
    pub fp: T,

    pub a: MemoryReadWriteCols<T>,
    pub b: MemoryReadWriteCols<T>,
    pub c: MemoryReadWriteCols<T>,

    pub instruction: InstructionCols<T>,

    // c = a + b;
    pub add_scratch: T,

    // c = a - b;
    pub sub_scratch: T,

    // c = a * b;
    pub mul_scratch: T,

    // ext(c) = ext(a) + ext(b);
    pub add_ext_scratch: Word<T>,

    // ext(c) = ext(a) - ext(b);
    pub sub_ext_scratch: Word<T>,

    // ext(c) = ext(a) * ext(b);
    pub mul_ext_scratch: Word<T>,

    // c = a == b;
    pub a_eq_b: IsZeroOperation<T>,

    pub is_real: T,
}

#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
pub struct InstructionCols<T> {
    pub opcode: T,
    pub op_a: T,
    pub op_b: T,
    pub op_c: T,
    pub imm_b: T,
    pub imm_c: T,
}
