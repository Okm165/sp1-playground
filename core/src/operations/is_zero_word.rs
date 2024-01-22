//! An operation to check if the input word is 0.
//!
//! This is bijective (i.e., returns 1 if and only if the input is 0). It is also worth noting that
//! this operation doesn't do a range check.
use core::borrow::Borrow;
use core::borrow::BorrowMut;
use p3_air::AirBuilder;
use p3_field::Field;
use std::mem::size_of;
use valida_derive::AlignedBorrow;

use crate::air::CurtaAirBuilder;
use crate::air::Word;
use crate::disassembler::WORD_SIZE;

use super::IsZeroOperation;

/// A set of columns needed to compute whether the given word is 0.
#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct IsZeroWordOperation<T> {
    /// `IsZeroOperation` to check if each byte in the input word is zero.
    pub is_zero_byte: [IsZeroOperation<T>; WORD_SIZE],

    /// A boolean flag indicating whether the lower word (the bottom 16 bits of the input) is 0.
    /// This equals `is_zero_byte[0] * is_zero_byte[1]`.
    pub is_lower_half_zero: T,

    /// A boolean flag indicating whether the upper word (the top 16 bits of the input) is 0. This
    /// equals `is_zero_byte[2] * is_zero_byte[3]`.
    pub is_upper_half_zero: T,

    /// A boolean flag indicating whether the word is zero. This equals `is_zero_byte[0] * ... *
    /// is_zero_byte[WORD_SIZE - 1]`.
    pub result: T,
}

impl<F: Field> IsZeroWordOperation<F> {
    pub fn populate(&mut self, a_u32: u32) -> u32 {
        let a = a_u32.to_le_bytes();
        for i in 0..WORD_SIZE {
            self.is_zero_byte[i].populate(a[i] as u32);
        }
        self.is_lower_half_zero = self.is_zero_byte[0].result * self.is_zero_byte[1].result;
        self.is_upper_half_zero = self.is_zero_byte[2].result * self.is_zero_byte[3].result;
        self.result = F::from_bool(a_u32 == 0);
        (a_u32 == 0) as u32
    }

    pub fn eval<AB: CurtaAirBuilder>(
        builder: &mut AB,
        a: Word<AB::Var>,
        cols: IsZeroWordOperation<AB::Var>,
        is_real: AB::Var,
    ) {
        // Calculate whether each byte is 0.
        for i in 0..WORD_SIZE {
            IsZeroOperation::<AB::F>::eval(builder, a[i], cols.is_zero_byte[i], is_real);
        }

        // From here, we only assert when is_real is true.
        builder.assert_bool(is_real);
        let mut builder_is_real = builder.when(is_real);

        // Calculate is_upper_half_zero and is_lower_half_zero and finally the result.
        builder_is_real.assert_bool(cols.is_lower_half_zero);
        builder_is_real.assert_bool(cols.is_upper_half_zero);
        builder_is_real.assert_bool(cols.result);
        builder_is_real.assert_eq(
            cols.is_lower_half_zero,
            cols.is_zero_byte[0].result * cols.is_zero_byte[1].result,
        );
        builder_is_real.assert_eq(
            cols.is_upper_half_zero,
            cols.is_zero_byte[2].result * cols.is_zero_byte[3].result,
        );
        builder_is_real.assert_eq(
            cols.result,
            cols.is_lower_half_zero * cols.is_upper_half_zero,
        );
    }
}
