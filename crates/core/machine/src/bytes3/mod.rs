pub mod air;
pub mod columns;
pub mod trace;

use self::columns::{Byte3PreprocessedCols, NUM_BYTE3_PREPROCESSED_COLS};
use crate::{bytes3::trace::NUM_ROWS, utils::zeroed_f_vec};
use core::borrow::BorrowMut;
use itertools::Itertools;
use p3_field::Field;
use p3_matrix::dense::RowMajorMatrix;
use sp1_core_executor::Byte3Opcode;
use std::marker::PhantomData;

/// The number of different byte operations.
pub const NUM_BYTE3_OPS: usize = 3;

/// A chip for computing byte operations.
///
/// The chip contains a preprocessed table of all possible byte operations. Other chips can then
/// use lookups into this table to compute their own operations.
#[derive(Debug, Clone, Copy, Default)]
pub struct Byte3Chip<F>(PhantomData<F>);

impl<F: Field> Byte3Chip<F> {
    /// Creates the preprocessed byte trace.
    ///
    /// This function returns a `trace` which is a matrix containing all possible byte operations.
    pub fn trace() -> RowMajorMatrix<F> {
        // The trace containing all values, with all multiplicities set to zero.
        let mut initial_trace = RowMajorMatrix::new(
            zeroed_f_vec(NUM_ROWS * NUM_BYTE3_PREPROCESSED_COLS),
            NUM_BYTE3_PREPROCESSED_COLS,
        );

        // Record all the necessary operations for each byte lookup.
        let opcodes = Byte3Opcode::all();

        // Iterate over all options for pairs of bytes `a` and `b`.
        for (row_index, (a, b)) in (0..=u8::MAX).cartesian_product(0..=u8::MAX).enumerate() {
            let a = a as u8;
            let b = b as u8;
            let col: &mut Byte3PreprocessedCols<F> = initial_trace.row_mut(row_index).borrow_mut();

            // Set the values of `a`, `b` and `c`.
            col.a = F::from_canonical_u8(a);
            col.b = F::from_canonical_u8(b);
            col.c = (0..=u8::MAX)
                .map(F::from_canonical_u8)
                .collect::<Vec<F>>()
                .try_into()
                .expect("Expected a Vec of length 256");

            // Iterate over all operations for results and updating the table map.
            for (opcode, c) in opcodes.iter().cartesian_product(0..=u8::MAX) {
                match opcode {
                    Byte3Opcode::XOR3 => {
                        let xor3 = a ^ b ^ c;
                        col.xor3[c as usize] = F::from_canonical_u8(xor3);
                    }
                    Byte3Opcode::CH => {
                        let ch = (a & b) ^ (!a & c);
                        col.ch[c as usize] = F::from_canonical_u8(ch);
                    }
                    Byte3Opcode::MAJ => {
                        let maj = (a & b) ^ (a & c) ^ (b & c);
                        col.maj[c as usize] = F::from_canonical_u8(maj);
                    }
                };
            }
        }

        initial_trace
    }
}

#[cfg(test)]
mod tests {
    use p3_baby_bear::BabyBear;
    use std::time::Instant;

    use super::*;

    #[test]
    pub fn test_trace_and_map() {
        let start = Instant::now();
        Byte3Chip::<BabyBear>::trace();
        println!("trace and map: {:?}", start.elapsed());
    }
}
