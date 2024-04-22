use crate::air::Block;
use sp1_derive::AlignedBorrow;

#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct AluCols<T> {
    pub ext_a: Block<T>,

    pub ext_b: Block<T>,

    // Used for div operations.
    pub inverse_scratch: Block<T>,
}
