//! Division and remainder verification.
//!
//! b = c * quotient + remainder where the signs of b and remainder match.
//!
//! Implementation:
//!
//! # Use the multiplication ALU table. result is 64 bits.
//! result = quotient * c.
//!
//! # Add sign-extended remainder to result. Propagate carry to handle overflow within bytes.
//! base = pow(2, 8)
//! carry = 0
//! for i in range(8):
//!     x = result[i] + remainder[i] + carry
//!     result[i] = x % base
//!     carry = x // base
//!
//! # The number represented by c * quotient + remainder in 64 bits must equal b in 32 bits.
//!
//! # Assert the lower 32 bits of result match b.
//! assert result[0..4] == b[0..4]
//!
//! # Assert the upper 32 bits of result match the sign of b.
//! if (b == -2^{31}) and (c == -1):
//!     # This is the only exception as this is the only case where it overflows.
//!     assert result[4..8] == [0, 0, 0, 0]
//! elif b < 0:
//!     assert result[4..8] == [0xff, 0xff, 0xff, 0xff]
//! else:
//!     assert result[4..8] == [0, 0, 0, 0]
//!
//! # Check a = quotient or remainder.
//! assert a == (quotient if opcode == division else remainder)
//!
//! # remainder and b must have the same sign.
//! if remainder < 0:
//!     assert b <= 0
//! if remainder > 0:
//!     assert b >= 0
//!
//! # abs(remainder) < abs(c)
//! if c < 0:
//!    assert c < remainder <= 0
//! elif c > 0:
//!    assert 0 <= remainder < c
//!
//! if division_by_0:
//!     # if division by 0, then quotient = 0xffffffff per RISC-V spec. This needs special care since
//!    # b = 0 * quotient + b is satisfied by any quotient.
//!    assert quotient = 0xffffffff

use core::borrow::{Borrow, BorrowMut};
use core::mem::{size_of, transmute};
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::AbstractField;
use p3_field::PrimeField;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::MatrixRowSlices;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use valida_derive::AlignedBorrow;

use crate::air::{CurtaAirBuilder, Word};
use crate::disassembler::WORD_SIZE;
use crate::runtime::{Opcode, Segment};
use crate::utils::{pad_to_power_of_two, Chip};

pub const NUM_DIVREM_COLS: usize = size_of::<DivRemCols<u8>>();

const BYTE_SIZE: usize = 8;

const LONG_WORD_SIZE: usize = 2 * WORD_SIZE;

fn get_msb(a: u32) -> u8 {
    ((a >> 31) & 1) as u8
}

/// The column layout for the chip.
#[derive(AlignedBorrow, Default, Debug)]
#[repr(C)]
pub struct DivRemCols<T> {
    /// The output operand.
    pub a: Word<T>,
    /// The first input operand.
    pub b: Word<T>,

    /// The second input operand.
    pub c: Word<T>,

    /// Results of dividing `b` by `c`.
    pub quotient: Word<T>,

    /// Remainder when dividing `b` by `c`.
    pub remainder: Word<T>,

    /// The result of `c * quotient`.
    pub c_times_quotient: [T; LONG_WORD_SIZE],

    /// Carry propagated when adding `remainder` by `c * quotient`.
    pub carry: [T; LONG_WORD_SIZE],

    /// Flag to indicate division by 0.
    pub division_by_0: T,

    /// The inverse of `c[0] + c[1] + c[2] + c[3]``, used to verify `division_by_0`.
    pub c_limb_sum_inverse: T,

    pub is_divu: T,
    pub is_remu: T,
    pub is_rem: T,
    pub is_div: T,

    /// Flag to indicate whether the division operation overflows.
    ///
    /// Overflow occurs in a specific case of signed 32-bit integer division: when `b` is the
    /// minimum representable value (`-2^31`, the smallest negative number) and `c` is `-1`. In this
    /// case, the division result exceeds the maximum positive value representable by a 32-bit
    /// signed integer.
    pub is_overflow: T,

    /// The most significant bit of `b`.
    pub b_msb: T,

    /// The most significant bit of remainder.
    pub rem_msb: T,

    /// Flag to indicate whether `b` is negative.
    pub b_neg: T,

    /// Flag to indicate whether `rem_neg` is negative.
    pub rem_neg: T,

    /// Selector to know whether this row is enabled.
    pub is_real: T,
}

/// A chip that implements addition for the opcodes DIV/REM.
pub struct DivRemChip;

impl DivRemChip {
    pub fn new() -> Self {
        Self {}
    }
}

fn is_signed_operation(opcode: Opcode) -> bool {
    opcode == Opcode::DIV || opcode == Opcode::REM
}

fn get_quotient_and_remainder(b: u32, c: u32, opcode: Opcode) -> (u32, u32) {
    if c == 0 {
        // When c is 0, the quotient is 2^32 - 1 and the remainder is b
        // regardless of whether we perform signed or unsigned division.
        (0xffff_ffff, b)
    } else if is_signed_operation(opcode) {
        (
            (b as i32).wrapping_div(c as i32) as u32,
            (b as i32).wrapping_rem(c as i32) as u32,
        )
    } else {
        (
            (b as u32).wrapping_div(c as u32) as u32,
            (b as u32).wrapping_rem(c as u32) as u32,
        )
    }
}

impl<F: PrimeField> Chip<F> for DivRemChip {
    fn generate_trace(&self, segment: &mut Segment) -> RowMajorMatrix<F> {
        // Generate the trace rows for each event.
        let rows = segment
            .divrem_events
            .par_iter()
            .map(|event| {
                assert!(
                    event.opcode == Opcode::DIVU
                        || event.opcode == Opcode::REMU
                        || event.opcode == Opcode::REM
                        || event.opcode == Opcode::DIV
                );
                // **TODO** Remove all the printf statements & asserts for debugging purposes.
                let mut row = [F::zero(); NUM_DIVREM_COLS];
                let cols: &mut DivRemCols<F> = unsafe { transmute(&mut row) };
                let a_word = event.a.to_le_bytes();
                let b_word = event.b.to_le_bytes();
                let c_word = event.c.to_le_bytes();
                cols.a = Word(a_word.map(F::from_canonical_u8));
                cols.b = Word(b_word.map(F::from_canonical_u8));
                cols.c = Word(c_word.map(F::from_canonical_u8));
                cols.is_real = F::one();
                cols.is_divu = F::from_bool(event.opcode == Opcode::DIVU);
                cols.is_remu = F::from_bool(event.opcode == Opcode::REMU);
                cols.is_div = F::from_bool(event.opcode == Opcode::DIV);
                cols.is_rem = F::from_bool(event.opcode == Opcode::REM);
                if event.c == 0 {
                    cols.division_by_0 = F::one();
                } else {
                    let c_limb_sum = cols.c[0] + cols.c[1] + cols.c[2] + cols.c[3];
                    cols.c_limb_sum_inverse = F::inverse(&c_limb_sum);
                    println!("c_limb_sum: {}", c_limb_sum);
                    println!("c_limb_sum_inverse: {}", cols.c_limb_sum_inverse);
                    println!(
                        "c_limb_sum * c_limb_sum_inverse: {}",
                        c_limb_sum * cols.c_limb_sum_inverse
                    );
                }
                let (quotient, remainder) =
                    get_quotient_and_remainder(event.b, event.c, event.opcode);
                println!(
                    "b: {}, c: {}, quotient: {}, remainder: {}",
                    event.b, event.c, quotient, remainder
                );

                cols.quotient = Word(quotient.to_le_bytes().map(F::from_canonical_u8));
                cols.remainder = Word(remainder.to_le_bytes().map(F::from_canonical_u8));
                cols.rem_msb = F::from_canonical_u8(get_msb(remainder));
                cols.b_msb = F::from_canonical_u8(get_msb(event.b));
                if is_signed_operation(event.opcode) {
                    cols.rem_neg = cols.rem_msb;
                    cols.b_neg = cols.b_msb;
                    cols.is_overflow =
                        F::from_bool(event.b as i32 == i32::MIN && event.c as i32 == -1);
                }

                let base = 1 << BYTE_SIZE;

                // print quotient and event.c
                println!("quotient: {}", quotient);
                println!("event.c : {}", quotient);
                if is_signed_operation(event.opcode) {
                    println!(
                        "b = quotient * c + remainder, {} = {} * {} + {}, => {}",
                        event.b as i32,
                        quotient as i32,
                        event.c as i32,
                        remainder as i32,
                        event.b as i32
                            == (quotient as i32)
                                .wrapping_mul(event.c as i32)
                                .wrapping_add(remainder as i32)
                    );
                }

                let c_times_quotient = {
                    if is_signed_operation(event.opcode) {
                        println!("quotient as i32 = {}", quotient as i32);
                        println!("event.c as i32 = {}", event.c as i32);
                        println!("quotient as i32 as i64 = {}", (quotient as i32) as i64);
                        println!("event.c as i32 as i64 = {}", (event.c as i32) as i64);
                        (((quotient as i32) as i64) * ((event.c as i32) as i64)).to_le_bytes()
                    } else {
                        ((quotient as u64) * (event.c as u64)).to_le_bytes()
                    }
                };

                cols.c_times_quotient = c_times_quotient.map(F::from_canonical_u8);

                let remainder_bytes = {
                    if is_signed_operation(event.opcode) {
                        ((remainder as i32) as i64).to_le_bytes()
                    } else {
                        (remainder as u64).to_le_bytes()
                    }
                };

                let mut result = [0u32; LONG_WORD_SIZE];

                // Add remainder to product.
                let mut carry = 0u32;
                let mut carry_ary = [0u32; LONG_WORD_SIZE];
                for i in 0..LONG_WORD_SIZE {
                    let x = c_times_quotient[i] as u32 + remainder_bytes[i] as u32 + carry;
                    result[i] = x % base;
                    carry = x / base;
                    cols.carry[i] = F::from_canonical_u32(carry);
                    carry_ary[i] = carry;
                }

                println!("carry_ary: {:#?}", carry_ary);
                println!("c_times_quotient: {:#?}", c_times_quotient);
                println!("remainder_bytes: {:#?}", remainder_bytes);

                for i in 0..LONG_WORD_SIZE {
                    let mut v = c_times_quotient[i] as u32 + remainder_bytes[i] as u32
                        - carry_ary[i] * base;
                    if i > 0 {
                        v += carry_ary[i - 1];
                    }
                    if i < WORD_SIZE {
                        debug_assert_eq!(v, b_word[i] as u32);
                    }
                }

                // The lower 4 bytes of the result must match the corresponding bytes in b.
                // result = c * quotient + remainder, so it must equal b.
                for i in 0..WORD_SIZE {
                    debug_assert_eq!(b_word[i] as u32, result[i]);
                }

                println!("{:#?}", cols);
                row
            })
            .collect::<Vec<_>>();

        // Convert the trace to a row major matrix.
        let mut trace = RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_DIVREM_COLS,
        );

        // Pad the trace to a power of two.
        pad_to_power_of_two::<NUM_DIVREM_COLS, F>(&mut trace.values);
        // Create the template for the padded rows. These are fake rows that don't fail on some
        // sanity checks.
        let padded_row_template = {
            let mut row = [F::zero(); NUM_DIVREM_COLS];
            let cols: &mut DivRemCols<F> = unsafe { transmute(&mut row) };
            // 0 divided by 1. quotient = remainder = 0.
            cols.is_divu = F::one();
            cols.c[0] = F::one();
            cols.c_limb_sum_inverse = F::one();

            row
        };
        debug_assert!(padded_row_template.len() == NUM_DIVREM_COLS);
        for i in segment.divrem_events.len() * NUM_DIVREM_COLS..trace.values.len() {
            trace.values[i] = padded_row_template[i % NUM_DIVREM_COLS];
        }

        println!("{:?}", trace.values);
        trace
    }
}

impl<F> BaseAir<F> for DivRemChip {
    fn width(&self) -> usize {
        NUM_DIVREM_COLS
    }
}

impl<AB> Air<AB> for DivRemChip
where
    AB: CurtaAirBuilder,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local: &DivRemCols<AB::Var> = main.row_slice(0).borrow();
        let base = AB::F::from_canonical_u32(1 << 8);
        let one: AB::Expr = AB::F::one().into();
        let zero: AB::Expr = AB::F::zero().into();

        // Calculate whether b, remainder, and c are negative.
        {
            // Negative if and only if op code is signed & MSB = 1
            let is_signed_type = local.is_div + local.is_rem;
            let b_neg = is_signed_type.clone() * local.b_msb;
            let rem_neg = is_signed_type.clone() * local.rem_msb;
            builder.assert_eq(b_neg.clone(), local.b_neg);
            builder.assert_eq(rem_neg.clone(), local.rem_neg);
        }

        // Use the mul table to compute c * quotient and compare it to local.c_times_quotient.
        {
            let lower_half: [AB::Expr; 4] = [
                local.c_times_quotient[0].into(),
                local.c_times_quotient[1].into(),
                local.c_times_quotient[2].into(),
                local.c_times_quotient[3].into(),
            ];

            // The lower 4 bytes of c_times_quotient must match the lower 4 bytes of (c * quotient)
            builder.send_alu(
                AB::Expr::from_canonical_u32(Opcode::MUL as u32),
                Word(lower_half),
                local.quotient.clone(),
                local.c.clone(),
                one.clone(),
            );

            // 1 = 0 * 0, which should fail.
            builder.send_alu(
                AB::Expr::from_canonical_u32(Opcode::MUL as u32),
                Word([zero.clone(), zero.clone(), zero.clone(), one.clone()]),
                Word([zero.clone(), zero.clone(), zero.clone(), zero.clone()]),
                Word([zero.clone(), zero.clone(), zero.clone(), zero.clone()]),
                one.clone(),
            );

            let opcode_for_upper_half = {
                let mulh = AB::Expr::from_canonical_u32(Opcode::MULH as u32);
                let mulhu = AB::Expr::from_canonical_u32(Opcode::MULHU as u32);
                let is_signed = local.is_div + local.is_rem;
                let is_unsigned = local.is_divu + local.is_remu;
                is_signed * mulh + is_unsigned * mulhu
            };

            let upper_half: [AB::Expr; 4] = [
                local.c_times_quotient[4].into(),
                local.c_times_quotient[5].into(),
                local.c_times_quotient[6].into(),
                local.c_times_quotient[7].into(),
            ];

            builder.send_alu(
                opcode_for_upper_half,
                Word(upper_half),
                local.quotient.clone(),
                local.c.clone(),
                one.clone(),
            );
        }

        // TODO: calculate is_overflow. is_overflow = is_equal(b, -2^{31}) * is_equal(c, -1)

        // Add remainder to product c * quotient, and compare it to b.
        {
            let sign_extension = local.rem_neg.clone() * AB::F::from_canonical_u32(0xff);
            let mut c_times_quotient_plus_remainder: Vec<AB::Expr> =
                vec![AB::F::zero().into(); LONG_WORD_SIZE];
            for i in 0..LONG_WORD_SIZE {
                c_times_quotient_plus_remainder[i] = local.c_times_quotient[i].into();

                // Add remainder.
                if i < WORD_SIZE {
                    c_times_quotient_plus_remainder[i] += local.remainder[i].into();
                } else {
                    // If rem is negative, add 0xff to the upper 4 bytes.
                    c_times_quotient_plus_remainder[i] += sign_extension.clone();
                }

                // Propagate carry.
                c_times_quotient_plus_remainder[i] -= local.carry[i].clone() * base.clone();
                if i > 0 {
                    c_times_quotient_plus_remainder[i] += local.carry[i - 1].into();
                }
            }

            for i in 0..LONG_WORD_SIZE {
                // Compare v to b[i].
                if i < WORD_SIZE {
                    // The lower 4 bytes of the result must match the corresponding bytes in b.
                    builder.when(local.is_real).assert_eq(
                        local.b[i].clone(),
                        c_times_quotient_plus_remainder[i].clone(),
                    );
                } else {
                    // The upper 4 bytes must reflect the sign of b in two's complement:
                    // - All 1s (0xff) for negative b.
                    // - All 0s for non-negative b.
                    let not_overflow = one.clone() - local.is_overflow.clone();
                    builder
                        .when(not_overflow.clone())
                        .when(local.b_neg)
                        .assert_eq(
                            c_times_quotient_plus_remainder[i].clone(),
                            AB::F::from_canonical_u32(0xff),
                        );
                    builder
                        .when(not_overflow.clone())
                        .when(one.clone() - local.b_neg)
                        .assert_eq(c_times_quotient_plus_remainder[i].clone(), zero.clone());

                    // The only exception to the upper-4-byte check is the overflow case.
                    builder
                        .when(local.is_overflow.clone())
                        .assert_eq(c_times_quotient_plus_remainder[i].clone(), zero.clone());
                }
            }
        }

        // a must equal remainder or quotient depending on the opcode.
        for i in 0..WORD_SIZE {
            builder
                .when(local.is_divu + local.is_div)
                .assert_eq(local.quotient[i], local.a[i]);
            builder
                .when(local.is_remu + local.is_rem)
                .assert_eq(local.remainder[i], local.a[i]);
        }

        // remainder and b must have the same sign. Due to the intricate nature of sign logic in ZK,
        // we will check a slightly stronger condition:
        //
        // 1. If remainder < 0, then b < 0.
        // 2. If remainder > 0, then b >= 0.
        {
            // A number is 0 if and only if the sum of the 4 limbs equals to 0.
            let mut rem_byte_sum = zero.clone();
            let mut b_byte_sum = zero.clone();
            for i in 0..WORD_SIZE {
                rem_byte_sum += local.remainder[i].into();
                b_byte_sum += local.b[i].into();
            }

            // 1. If remainder < 0, then b < 0.
            builder
                .when(local.rem_neg) // rem is negative.
                .assert_one(local.b_neg); // b is negative.

            // 2. If remainder > 0, then b >= 0.
            builder
                .when(rem_byte_sum.clone()) // remainder is nonzero.
                .when(one.clone() - local.rem_neg) // rem is not negative.
                .assert_zero(local.b_neg); // b is not negative.
        }

        // When division by 0, quotient must be 0xffffffff per RISC-V spec.
        {
            // If c = 0, then 1 - c_limb_sum * c_limb_sum_inverse is nonzero.
            let c_limb_sum = local.c[0] + local.c[1] + local.c[2] + local.c[3];
            builder
                .when(one.clone() - c_limb_sum * local.c_limb_sum_inverse)
                .assert_eq(local.division_by_0, one.clone());

            for i in 0..WORD_SIZE {
                builder
                    .when(local.division_by_0.clone())
                    .when(local.is_divu.clone() + local.is_div.clone())
                    .assert_eq(local.quotient[i], AB::F::from_canonical_u32(0xff));
            }
        }

        // TODO: Range check remainder. (i.e., 0 <= |remainder| < |c| when not division_by_0)
        {
            // Use the LT ALU table.
        }

        // TODO: Use lookup to constrain the MSBs.
        {
            let msb_pairs = [
                (local.b_msb.clone(), local.b[WORD_SIZE - 1].clone()),
                (
                    local.rem_msb.clone(),
                    local.remainder[WORD_SIZE - 1].clone(),
                ),
            ];
            for msb_pair in msb_pairs.iter() {
                let _msb = msb_pair.0.clone();
                let _byte = msb_pair.1.clone();
                // _msb must match _byte's msb.
            }
        }

        // TODO: Range check all the bytes.
        {
            let words = [local.a, local.b, local.c, local.quotient, local.remainder];
            let long_words = [local.c_times_quotient, local.carry];

            for word in words.iter() {
                for _byte in word.0.iter() {
                    // byte must be in [0, 255].
                }
            }

            for long_word in long_words.iter() {
                for _byte in long_word.iter() {
                    // byte must be in [0, 255].
                }
            }
        }

        // Check that the flags are boolean.
        {
            let bool_flags = [
                local.is_real,
                local.is_remu,
                local.is_divu,
                local.is_rem,
                local.is_div,
                local.b_neg,
                local.rem_neg,
                local.b_msb,
                local.rem_msb,
                local.division_by_0,
            ];

            for flag in bool_flags.iter() {
                builder.assert_bool(flag.clone());
            }
        }

        // Receive the arguments.
        {
            // Exactly one of the opcode flags must be on.
            builder.when(local.is_real).assert_eq(
                one.clone(),
                local.is_divu + local.is_remu + local.is_div + local.is_rem,
            );

            let opcode = {
                let divu: AB::Expr = AB::F::from_canonical_u32(Opcode::DIVU as u32).into();
                let remu: AB::Expr = AB::F::from_canonical_u32(Opcode::REMU as u32).into();
                let div: AB::Expr = AB::F::from_canonical_u32(Opcode::DIV as u32).into();
                let rem: AB::Expr = AB::F::from_canonical_u32(Opcode::REM as u32).into();

                local.is_divu * divu
                    + local.is_remu * remu
                    + local.is_div * div
                    + local.is_rem * rem
            };

            builder.receive_alu(opcode, local.a, local.b, local.c, local.is_real);
        }

        // A dummy constraint to keep the degree 3.
        builder.assert_zero(
            local.a[0] * local.b[0] * local.c[0] - local.a[0] * local.b[0] * local.c[0],
        )
    }
}

#[cfg(test)]
mod tests {
    use p3_challenger::DuplexChallenger;
    use p3_dft::Radix2DitParallel;
    use p3_field::Field;

    use p3_baby_bear::BabyBear;
    use p3_field::extension::BinomialExtensionField;
    use p3_fri::{FriBasedPcs, FriConfigImpl, FriLdt};
    use p3_keccak::Keccak256Hash;
    use p3_ldt::QuotientMmcs;
    use p3_matrix::dense::RowMajorMatrix;
    use p3_mds::coset_mds::CosetMds;
    use p3_merkle_tree::FieldMerkleTreeMmcs;
    use p3_poseidon2::{DiffusionMatrixBabybear, Poseidon2};
    use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher32};
    use p3_uni_stark::{prove, verify, StarkConfigImpl};
    use rand::thread_rng;

    use crate::{
        alu::AluEvent,
        runtime::{Opcode, Program, Runtime, Segment},
        utils::Chip,
    };
    use p3_commit::ExtensionMmcs;

    use super::DivRemChip;

    #[test]
    fn generate_trace() {
        let mut segment = Segment::default();
        segment.divrem_events = vec![AluEvent::new(0, Opcode::DIVU, 2, 17, 3)];
        let chip = DivRemChip::new();
        let trace: RowMajorMatrix<BabyBear> = chip.generate_trace(&mut segment);
        println!("{:?}", trace.values)
    }

    fn neg(a: u32) -> u32 {
        u32::MAX - a + 1
    }

    #[test]
    fn prove_babybear() {
        type Val = BabyBear;
        type Domain = Val;
        type Challenge = BinomialExtensionField<Val, 4>;
        type PackedChallenge = BinomialExtensionField<<Domain as Field>::Packing, 4>;

        type MyMds = CosetMds<Val, 16>;
        let mds = MyMds::default();

        type Perm = Poseidon2<Val, MyMds, DiffusionMatrixBabybear, 16, 5>;
        let perm = Perm::new_from_rng(8, 22, mds, DiffusionMatrixBabybear, &mut thread_rng());

        type MyHash = SerializingHasher32<Keccak256Hash>;
        let hash = MyHash::new(Keccak256Hash {});

        type MyCompress = CompressionFunctionFromHasher<Val, MyHash, 2, 8>;
        let compress = MyCompress::new(hash);

        type ValMmcs = FieldMerkleTreeMmcs<Val, MyHash, MyCompress, 8>;
        let val_mmcs = ValMmcs::new(hash, compress);

        type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;
        let challenge_mmcs = ChallengeMmcs::new(val_mmcs.clone());

        type Dft = Radix2DitParallel;
        let dft = Dft {};

        type Challenger = DuplexChallenger<Val, Perm, 16>;

        type Quotient = QuotientMmcs<Domain, Challenge, ValMmcs>;
        type MyFriConfig = FriConfigImpl<Val, Challenge, Quotient, ChallengeMmcs, Challenger>;
        let fri_config = MyFriConfig::new(40, challenge_mmcs);
        let ldt = FriLdt { config: fri_config };

        type Pcs = FriBasedPcs<MyFriConfig, ValMmcs, Dft, Challenger>;
        type MyConfig = StarkConfigImpl<Val, Challenge, PackedChallenge, Pcs, Challenger>;

        let pcs = Pcs::new(dft, val_mmcs, ldt);
        let config = StarkConfigImpl::new(pcs);
        let mut challenger = Challenger::new(perm.clone());

        let instructions = vec![];
        let program = Program::new(instructions, 0, 0);
        let mut runtime = Runtime::new(program);
        let mut divrem_events: Vec<AluEvent> = Vec::new();

        let divrems: Vec<(Opcode, u32, u32, u32)> = vec![
            (Opcode::DIVU, 3, 20, 6),
            (Opcode::DIVU, 715827879, neg(20), 6),
            (Opcode::DIVU, 0, 20, neg(6)),
            (Opcode::DIVU, 0, neg(20), neg(6)),
            (Opcode::DIVU, 1 << 31, 1 << 31, 1),
            (Opcode::DIVU, 0, 1 << 31, neg(1)),
            (Opcode::DIVU, u32::MAX, 1 << 31, 0),
            (Opcode::DIVU, u32::MAX, 1, 0),
            (Opcode::DIVU, u32::MAX, 0, 0),
            (Opcode::REMU, 4, 18, 7),
            (Opcode::REMU, 6, neg(20), 11),
            (Opcode::REMU, 23, 23, neg(6)),
            (Opcode::REMU, neg(21), neg(21), neg(11)),
            (Opcode::REMU, 5, 5, 0),
            (Opcode::REMU, neg(1), neg(1), 0),
            (Opcode::REMU, 0, 0, 0),
            (Opcode::REM, 7, 16, 9),
            (Opcode::REM, neg(4), neg(22), 6),
            (Opcode::REM, 1, 25, neg(3)),
            (Opcode::REM, neg(2), neg(22), neg(4)),
            (Opcode::REM, 0, 873, 1),
            (Opcode::REM, 0, 873, neg(1)),
            (Opcode::REM, 5, 5, 0),
            (Opcode::REM, neg(5), neg(5), 0),
            (Opcode::REM, 0, 0, 0),
            (Opcode::REM, 0, 0x80000001, neg(1)),
            (Opcode::DIV, 3, 18, 6),
            (Opcode::DIV, neg(6), neg(24), 4),
            (Opcode::DIV, neg(2), 16, neg(8)),
            (Opcode::DIV, neg(1), 0, 0),
            (Opcode::DIV, 1 << 31, 1 << 31, neg(1)),
            (Opcode::REM, 0, 1 << 31, neg(1)),
        ];
        for t in divrems.iter() {
            divrem_events.push(AluEvent::new(0, t.0, t.1, t.2, t.3));
        }

        // Append more events until we have 1000 tests.
        for _ in 0..(1000 - divrems.len()) {
            //            divrem_events.push(AluEvent::new(0, Opcode::DIVU, 1, 1, 1));
        }

        let mut segment = Segment::default();
        segment.divrem_events = divrem_events;
        let chip = DivRemChip::new();
        let trace: RowMajorMatrix<BabyBear> = chip.generate_trace(&mut segment);
        let proof = prove::<MyConfig, _>(&config, &chip, &mut challenger, trace);

        let mut challenger = Challenger::new(perm);
        verify(&config, &chip, &mut challenger, &proof).unwrap();
    }
}
