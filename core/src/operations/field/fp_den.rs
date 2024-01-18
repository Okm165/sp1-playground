use super::params::NUM_WITNESS_LIMBS;
use super::params::{convert_polynomial, convert_vec, FieldParameters, Limbs};
use super::util::{compute_root_quotient_and_shift, split_u16_limbs_to_u8_limbs};
use super::util_air::eval_field_operation;
use crate::air::polynomial::Polynomial;
use crate::air::CurtaAirBuilder;
use core::borrow::{Borrow, BorrowMut};
use core::mem::size_of;
use num::BigUint;
use p3_baby_bear::BabyBear;
use p3_field::Field;
use std::fmt::Debug;
use valida_derive::AlignedBorrow;

/// A set of columns to compute `FpDen(a, b)` where a, b are field elements.
/// Right now the number of limbs is assumed to be a constant, although this could be macro-ed
/// or made generic in the future.
#[derive(Debug, Clone, AlignedBorrow)]
#[repr(C)]
pub struct FpDenCols<T> {
    /// The result of `a op b`, where a, b are field elements
    pub result: Limbs<T>,
    pub(crate) carry: Limbs<T>,
    pub(crate) witness_low: [T; NUM_WITNESS_LIMBS],
    pub(crate) witness_high: [T; NUM_WITNESS_LIMBS],
}

impl<F: Field> FpDenCols<F> {
    pub fn populate<P: FieldParameters>(
        &mut self,
        a: &BigUint,
        b: &BigUint,
        sign: bool,
    ) -> BigUint {
        /// TODO: This operation relies on `F` being a PrimeField32, but our traits do not
        /// support that. This is a hack, since we always use BabyBear, to get around that, but
        /// all operations using "PF" should use "F" in the future.
        type PF = BabyBear;

        let p = P::modulus();
        let minus_b_int = &p - b;
        let b_signed = if sign { b.clone() } else { minus_b_int };
        let denominator = (b_signed + 1u32) % &(p.clone());
        let den_inv = denominator.modpow(&(&p - 2u32), &p);
        let result = (a * &den_inv) % &p;
        debug_assert_eq!(&den_inv * &denominator % &p, BigUint::from(1u32));
        debug_assert!(result < p);

        let equation_lhs = if sign {
            b * &result + &result
        } else {
            b * &result + a
        };
        let equation_rhs = if sign { a.clone() } else { result.clone() };
        let carry = (&equation_lhs - &equation_rhs) / &p;
        debug_assert!(carry < p);
        debug_assert_eq!(&carry * &p, &equation_lhs - &equation_rhs);

        let p_a: Polynomial<PF> = P::to_limbs_field::<PF>(a).into();
        let p_b: Polynomial<PF> = P::to_limbs_field::<PF>(b).into();
        let p_p: Polynomial<PF> = P::to_limbs_field::<PF>(&p).into();
        let p_result: Polynomial<PF> = P::to_limbs_field::<PF>(&result).into();
        let p_carry: Polynomial<PF> = P::to_limbs_field::<PF>(&carry).into();

        // Compute the vanishing polynomial.
        let vanishing_poly = if sign {
            &p_b * &p_result + &p_result - &p_a - &p_carry * &p_p
        } else {
            &p_b * &p_result + &p_a - &p_result - &p_carry * &p_p
        };
        debug_assert_eq!(vanishing_poly.degree(), P::NB_WITNESS_LIMBS);

        let p_witness = compute_root_quotient_and_shift(
            &vanishing_poly,
            P::WITNESS_OFFSET,
            P::NB_BITS_PER_LIMB as u32,
        );
        let (p_witness_low, p_witness_high) = split_u16_limbs_to_u8_limbs(&p_witness);

        self.result = convert_polynomial(p_result);
        self.carry = convert_polynomial(p_carry);
        self.witness_low = convert_vec(p_witness_low).try_into().unwrap();
        self.witness_high = convert_vec(p_witness_high).try_into().unwrap();

        result
    }

    #[allow(unused_variables)]
    pub fn eval<AB: CurtaAirBuilder<F = F>, P: FieldParameters>(
        builder: &mut AB,
        cols: &FpDenCols<AB::Var>,
        a: &Limbs<AB::Var>,
        b: &Limbs<AB::Var>,
        sign: bool,
    ) {
        let p_a = (*a).clone().into();
        let p_b = (*b).clone().into();
        let p_result = cols.result.clone().into();
        let p_carry = cols.carry.clone().into();

        // Compute the vanishing polynomial:
        //      lhs(x) = sign * (b(x) * result(x) + result(x)) + (1 - sign) * (b(x) * result(x) + a(x))
        //      rhs(x) = sign * a(x) + (1 - sign) * result(x)
        //      lhs(x) - rhs(x) - carry(x) * p(x)
        let p_b_times_res = builder.poly_mul(&p_b, &p_result);
        let p_equation_lhs = if sign {
            builder.poly_add(&p_b_times_res, &p_result)
        } else {
            builder.poly_add(&p_b_times_res, &p_a)
        };
        let p_equation_rhs = if sign { p_a } else { p_result };

        let p_lhs_minus_rhs = builder.poly_sub(&p_equation_lhs, &p_equation_rhs);
        let p_limbs = builder.constant_poly(&Polynomial::from_iter(P::modulus_field_iter::<F>()));

        let mul_times_carry = builder.poly_mul(&p_carry, &p_limbs);
        let p_vanishing = builder.poly_sub(&p_lhs_minus_rhs, &mul_times_carry);

        let p_witness_low = cols.witness_low.iter().into();
        let p_witness_high = cols.witness_high.iter().into();

        eval_field_operation::<AB, P>(builder, &p_vanishing, &p_witness_low, &p_witness_high);
    }
}

#[cfg(test)]
mod tests {
    use num::BigUint;
    use p3_air::BaseAir;
    use p3_challenger::DuplexChallenger;
    use p3_dft::Radix2DitParallel;
    use p3_field::Field;

    use super::{FpDenCols, FpOperation, Limbs};
    use crate::utils::pad_to_power_of_two;
    use crate::{
        air::CurtaAirBuilder,
        operations::field::params::{Ed25519BaseField, FieldParameters},
        runtime::Segment,
        utils::Chip,
    };
    use core::borrow::{Borrow, BorrowMut};
    use core::mem::{size_of, transmute};
    use p3_air::Air;
    use p3_baby_bear::BabyBear;
    use p3_commit::ExtensionMmcs;
    use p3_field::extension::BinomialExtensionField;
    use p3_fri::{FriBasedPcs, FriConfigImpl, FriLdt};
    use p3_keccak::Keccak256Hash;
    use p3_ldt::QuotientMmcs;
    use p3_matrix::dense::RowMajorMatrix;
    use p3_matrix::MatrixRowSlices;
    use p3_mds::coset_mds::CosetMds;
    use p3_merkle_tree::FieldMerkleTreeMmcs;
    use p3_poseidon2::{DiffusionMatrixBabybear, Poseidon2};
    use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher32};
    use p3_uni_stark::{prove, verify, StarkConfigImpl};
    use rand::thread_rng;
    use valida_derive::AlignedBorrow;

    #[derive(AlignedBorrow, Debug, Clone)]
    pub struct TestCols<T> {
        pub a: Limbs<T>,
        pub b: Limbs<T>,
        pub a_den_b: FpDenCols<T>,
    }

    pub const NUM_TEST_COLS: usize = size_of::<TestCols<u8>>();

    struct FpDenChip<P: FieldParameters> {
        pub sign: bool,
        pub _phantom: std::marker::PhantomData<P>,
    }

    impl<P: FieldParameters> FpDenChip<P> {
        pub fn new(sign: bool) -> Self {
            Self {
                sign,
                _phantom: std::marker::PhantomData,
            }
        }
    }

    impl<F: Field, P: FieldParameters> Chip<F> for FpDenChip<P> {
        fn generate_trace(&self, _: &mut Segment) -> RowMajorMatrix<F> {
            // TODO: ignore segment for now since we're just hardcoding this test case.
            let operands: Vec<(BigUint, BigUint)> = vec![
                (BigUint::from(0u32), BigUint::from(0u32)),
                (BigUint::from(1u32), BigUint::from(2u32)),
                (BigUint::from(4u32), BigUint::from(5u32)),
                (BigUint::from(10u32), BigUint::from(19u32)),
            ];
            let rows = operands
                .iter()
                .map(|(a, b)| {
                    let mut row = [F::zero(); NUM_TEST_COLS];
                    let cols: &mut TestCols<F> = unsafe { transmute(&mut row) };
                    cols.a = P::to_limbs_field::<F>(a);
                    cols.b = P::to_limbs_field::<F>(b);
                    cols.a_den_b.populate::<P>(a, b, self.sign);
                    row
                })
                .collect::<Vec<_>>();
            // Convert the trace to a row major matrix.
            let mut trace = RowMajorMatrix::new(
                rows.into_iter().flatten().collect::<Vec<_>>(),
                NUM_TEST_COLS,
            );

            // Pad the trace to a power of two.
            pad_to_power_of_two::<NUM_TEST_COLS, F>(&mut trace.values);

            trace
        }
    }

    impl<F: Field, P: FieldParameters> BaseAir<F> for FpDenChip<P> {
        fn width(&self) -> usize {
            NUM_TEST_COLS
        }
    }

    impl<AB, P: FieldParameters> Air<AB> for FpDenChip<P>
    where
        AB: CurtaAirBuilder,
    {
        fn eval(&self, builder: &mut AB) {
            let main = builder.main();
            let local: &TestCols<AB::Var> = main.row_slice(0).borrow();
            FpDenCols::eval::<AB, P>(builder, &local.a_den_b, &local.a, &local.b, true);

            // A dummy constraint to keep the degree 3.
            builder.assert_zero(
                local.a[0] * local.b[0] * local.a[0] - local.a[0] * local.b[0] * local.a[0],
            )
        }
    }

    #[test]
    fn generate_trace() {
        let mut segment = Segment::default();
        let chip: FpDenChip<Ed25519BaseField> = FpDenChip::new(true);
        let trace: RowMajorMatrix<BabyBear> = chip.generate_trace(&mut segment);
        println!("{:?}", trace.values)
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

        let mut segment = Segment::default();

        let chip: FpDenChip<Ed25519BaseField> = FpDenChip::new(true);
        let trace: RowMajorMatrix<BabyBear> = chip.generate_trace(&mut segment);
        let proof = prove::<MyConfig, _>(&config, &chip, &mut challenger, trace);

        let mut challenger = Challenger::new(perm);
        verify(&config, &chip, &mut challenger, &proof).unwrap();
    }
}
