use core::borrow::Borrow;
use core::borrow::BorrowMut;
use p3_air::AirBuilder;
use p3_field::Field;
use std::mem::size_of;
use valida_derive::AlignedBorrow;

use crate::air::CurtaAirBuilder;
use crate::air::Word;
use crate::air::WORD_SIZE;
use crate::bytes::ByteOpcode;
use crate::runtime::Segment;
use p3_field::AbstractField;

/// A set of columns needed to compute the add of five words.
#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct Add5Operation<T> {
    /// The result of `a + b + c + d + e`.
    pub value: Word<T>,

    /// Indicates if the carry for the `i`th digit is 0.
    pub is_carry_0: Word<T>,

    /// Indicates if the carry for the `i`th digit is 1.
    pub is_carry_1: Word<T>,

    /// Indicates if the carry for the `i`th digit is 2.
    pub is_carry_2: Word<T>,

    /// Indicates if the carry for the `i`th digit is 3.
    pub is_carry_3: Word<T>,

    /// Indicates if the carry for the `i`th digit is 4. The carry when adding 5 words is at most 4.
    pub is_carry_4: Word<T>,

    /// The carry for the `i`th digit.
    pub carry: Word<T>,
}

impl<F: Field> Add5Operation<F> {
    pub fn populate(&mut self, a_u32: u32, b_u32: u32, c_u32: u32, d_u32: u32, e_u32: u32) -> u32 {
        let expected = a_u32
            .wrapping_add(b_u32)
            .wrapping_add(c_u32)
            .wrapping_add(d_u32)
            .wrapping_add(e_u32);

        self.value = Word::from(expected);
        let a = a_u32.to_le_bytes();
        let b = b_u32.to_le_bytes();
        let c = c_u32.to_le_bytes();
        let d = d_u32.to_le_bytes();
        let e = e_u32.to_le_bytes();

        let base = 256;
        let mut carry = [0u8, 0u8, 0u8, 0u8, 0u8];
        for i in 0..WORD_SIZE {
            let mut res =
                (a[i] as u32) + (b[i] as u32) + (c[i] as u32) + (d[i] as u32) + (e[i] as u32);
            if i > 0 {
                res += carry[i - 1] as u32;
            }
            carry[i] = (res / base) as u8;
            self.is_carry_0[i] = F::from_bool(carry[i] == 0);
            self.is_carry_1[i] = F::from_bool(carry[i] == 1);
            self.is_carry_2[i] = F::from_bool(carry[i] == 2);
            self.is_carry_3[i] = F::from_bool(carry[i] == 3);
            self.is_carry_4[i] = F::from_bool(carry[i] == 4);
            self.carry[i] = F::from_canonical_u8(carry[i]);
            debug_assert!(carry[i] <= 4);
            debug_assert_eq!(self.value[i], F::from_canonical_u32(res % base));
        }

        expected
    }

    #[allow(unused_variables)]
    pub fn eval<AB: CurtaAirBuilder>(
        builder: &mut AB,
        a: Word<AB::Var>,
        b: Word<AB::Var>,
        c: Word<AB::Var>,
        d: Word<AB::Var>,
        e: Word<AB::Var>,
        is_real: AB::Var,
        cols: Add5Operation<AB::Var>,
    ) {
        builder.assert_bool(is_real);
        let mut builder_is_real = builder.when(is_real);

        // Each value in is_carry_{0,1,2,3,4} is 0 or 1, and exactly one of them is 1 per digit.
        {
            for i in 0..WORD_SIZE {
                builder_is_real.assert_bool(cols.is_carry_0[i]);
                builder_is_real.assert_bool(cols.is_carry_1[i]);
                builder_is_real.assert_bool(cols.is_carry_2[i]);
                builder_is_real.assert_bool(cols.is_carry_3[i]);
                builder_is_real.assert_bool(cols.is_carry_4[i]);
                builder_is_real.assert_eq(
                    cols.is_carry_0[i]
                        + cols.is_carry_1[i]
                        + cols.is_carry_2[i]
                        + cols.is_carry_3[i]
                        + cols.is_carry_4[i],
                    AB::Expr::one(),
                );
            }
        }

        // Calculates carry from is_carry_{0,1,2,3,4}.
        {
            let one = AB::Expr::one();
            let two = AB::F::from_canonical_u32(2);
            let three = AB::F::from_canonical_u32(3);
            let four = AB::F::from_canonical_u32(4);

            for i in 0..WORD_SIZE {
                builder_is_real.assert_eq(
                    cols.carry[i],
                    cols.is_carry_1[i] * one.clone()
                        + cols.is_carry_2[i] * two.clone()
                        + cols.is_carry_3[i] * three.clone()
                        + cols.is_carry_4[i] * four.clone(),
                );
            }
        }

        // Compare the sum and summands by looking at carry.
        {
            let base = AB::F::from_canonical_u32(256);
            // For each limb, assert that difference between the carried result and the non-carried
            // result is the product of carry and base.
            for i in 0..WORD_SIZE {
                let mut overflow = a[i] + b[i] + c[i] + d[i] + e[i] - cols.value[i];
                if i > 0 {
                    overflow += cols.carry[i - 1].into();
                }
                builder_is_real.assert_eq(cols.carry[i].clone() * base, overflow.clone());
            }
        }

        // Degree 3 constraint to avoid "OodEvaluationMismatch".
        builder.assert_zero(a[0] * b[0] * cols.value[0] - a[0] * b[0] * cols.value[0]);
    }
}
