use std::cmp::{max, min};

use crate::air::polynomial::Polynomial;

use num::{BigUint, One, Zero};
use p3_field::PrimeField32;

/// Computes the difference between `a` and `b` modulo `n`.
pub fn subtract_mod(a: &BigUint, b: &BigUint, n: &BigUint) -> BigUint {
    let a_mod = a % n;
    let b_mod = b % n;
    if a_mod < b_mod {
        (n + a_mod) - b_mod
    } else {
        a_mod - b_mod
    }
}
/// Computes the inverse of `a` modulo `n` using the same idea as the extended Euclidean algorithm.
/// See https://en.wikipedia.org/wiki/Extended_Euclidean_algorithm#Modular_integers
pub fn inverse_mod(a: &BigUint, n: &BigUint) -> BigUint {
    let mut t = BigUint::zero();
    let mut new_t = BigUint::one();
    let mut r = n.clone();
    let mut new_r = a.clone();

    while !new_r.is_zero() {
        let quotient = &r / &new_r;
        (t, new_t) = (new_t.clone(), subtract_mod(&t, &(&quotient * new_t), n));
        (r, new_r) = (new_r.clone(), subtract_mod(&r, &(&quotient * new_r), n));
    }

    // The GCD has to be 1 for a to be invertible.
    assert_eq!(r, BigUint::one());

    t
}

fn biguint_to_field<F: PrimeField32>(num: BigUint) -> F {
    let mut x = F::zero();
    let mut power = F::from_canonical_u32(1u32);
    let base = F::from_canonical_u64((1 << 32) % F::ORDER_U64);
    let digits = num.iter_u32_digits();
    for digit in digits.into_iter() {
        x += F::from_canonical_u32(digit) * power;
        power *= base;
    }
    x
}

/// Asserts that two polynomials are equal while ignoring leading 0's.
pub fn assert_eq_polynomial<F: PrimeField32>(p1: &Polynomial<F>, p2: &Polynomial<F>) {
    let coef1 = p1.coefficients();
    let coef2 = p2.coefficients();
    for i in 0..max(coef1.len(), coef2.len()) {
        if i < min(coef1.len(), coef2.len()) {
            assert_eq!(coef1[i], coef2[i]);
        } else if i < coef1.len() {
            assert_eq!(coef1[i], F::zero());
        } else {
            assert_eq!(coef2[i], F::zero());
        }
    }
}

#[inline]
pub fn compute_root_quotient_and_shift<F: PrimeField32>(
    p_vanishing: &Polynomial<F>,
    offset: usize,
    nb_bits_per_limb: u32,
) -> Vec<F> {
    // Evaluate the vanishing polynomial at x = 2^nb_bits_per_limb.
    let p_vanishing_eval = p_vanishing
        .coefficients()
        .iter()
        .enumerate()
        .map(|(i, x)| {
            biguint_to_field::<F>(BigUint::from(2u32).pow(nb_bits_per_limb * i as u32)) * *x
        })
        .sum::<F>();
    debug_assert_eq!(p_vanishing_eval, F::zero());

    // Compute the witness polynomial by witness(x) = vanishing(x) / (x - 2^nb_bits_per_limb).
    let root_monomial = F::from_canonical_u32(2u32.pow(nb_bits_per_limb));
    let p_quotient = p_vanishing.root_quotient(root_monomial);
    debug_assert_eq!(p_quotient.degree(), p_vanishing.degree() - 1);

    // Sanity Check #1: For all i, |w_i| < 2^20 to prevent overflows.
    let offset_u64 = offset as u64;
    for c in p_quotient.coefficients().iter() {
        debug_assert!(c.neg().as_canonical_u64() < offset_u64 || c.as_canonical_u64() < offset_u64);
    }

    // Sanity Check #2: w(x) * (x - 2^nb_bits_per_limb) = vanishing(x).
    let x_minus_root = Polynomial::<F>::from_coefficients_slice(&[-root_monomial, F::one()]);
    assert_eq_polynomial(&(&p_quotient * &x_minus_root), p_vanishing);

    // Shifting the witness polynomial to make it positive
    p_quotient
        .coefficients()
        .iter()
        .map(|x| *x + F::from_canonical_u64(offset_u64))
        .collect::<Vec<F>>()
}

#[inline]
pub fn split_u16_limbs_to_u8_limbs<F: PrimeField32>(slice: &[F]) -> (Vec<F>, Vec<F>) {
    (
        slice
            .iter()
            .map(|x| x.as_canonical_u64() as u8)
            .map(|x| F::from_canonical_u8(x))
            .collect(),
        slice
            .iter()
            .map(|x| (x.as_canonical_u64() >> 8) as u8)
            .map(|x| F::from_canonical_u8(x))
            .collect(),
    )
}
