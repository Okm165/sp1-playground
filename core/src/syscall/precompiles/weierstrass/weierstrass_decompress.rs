use core::borrow::{Borrow, BorrowMut};
use core::mem::size_of;
use std::fmt::Debug;

use generic_array::GenericArray;
use num::{BigUint, Zero};
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::AbstractField;
use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;
use sp1_derive::AlignedBorrow;
use std::marker::PhantomData;
use typenum::Unsigned;

use crate::air::{BaseAirBuilder, MachineAir, SP1AirBuilder};
use crate::bytes::event::ByteRecord;
use crate::memory::MemoryReadCols;
use crate::memory::MemoryReadWriteCols;
use crate::operations::field::field_op::FieldOpCols;
use crate::operations::field::field_op::FieldOperation;
use crate::operations::field::field_sqrt::FieldSqrtCols;
use crate::operations::field::params::{limbs_from_vec, FieldParameters, NumWords};
use crate::operations::field::params::{Limbs, NumLimbs};
use crate::operations::field::range::FieldLtCols;
use crate::runtime::ExecutionRecord;
use crate::runtime::Program;
use crate::runtime::Syscall;
use crate::runtime::SyscallCode;
use crate::syscall::precompiles::create_ec_decompress_event;
use crate::syscall::precompiles::SyscallContext;
use crate::utils::ec::weierstrass::bls12_381::bls12381_sqrt;
use crate::utils::ec::weierstrass::secp256k1::secp256k1_sqrt;
use crate::utils::ec::weierstrass::WeierstrassParameters;
use crate::utils::ec::CurveType;
use crate::utils::ec::EllipticCurve;
use crate::utils::limbs_from_access;
use crate::utils::limbs_from_prev_access;
use crate::utils::{bytes_to_words_le_vec, pad_rows};

pub const fn num_weierstrass_decompress_cols<P: FieldParameters + NumWords>() -> usize {
    size_of::<WeierstrassDecompressCols<u8, P>>()
}

/// A set of columns to compute `WeierstrassDecompress` that decompresses a point on a Weierstrass
/// curve.
#[derive(Debug, Clone, AlignedBorrow)]
#[repr(C)]
pub struct WeierstrassDecompressCols<T, P: FieldParameters + NumWords> {
    pub is_real: T,
    pub shard: T,
    pub channel: T,
    pub clk: T,
    pub nonce: T,
    pub ptr: T,
    pub is_odd: T,
    pub x_access: GenericArray<MemoryReadCols<T>, P::WordsFieldElement>,
    pub y_access: GenericArray<MemoryReadWriteCols<T>, P::WordsFieldElement>,
    pub(crate) range_x: FieldLtCols<T, P>,
    pub(crate) x_2: FieldOpCols<T, P>,
    pub(crate) x_3: FieldOpCols<T, P>,
    pub(crate) x_3_plus_b: FieldOpCols<T, P>,
    pub(crate) y: FieldSqrtCols<T, P>,
    pub(crate) neg_y: FieldOpCols<T, P>,
}

pub enum SignChoiceRule {
    LeastSignificantBit,
    Lexicographic,
}

pub struct WeierstrassDecompressChip<E> {
    sign_rule: SignChoiceRule,
    _marker: PhantomData<E>,
}

impl<E: EllipticCurve> Syscall for WeierstrassDecompressChip<E> {
    fn execute(&self, rt: &mut SyscallContext, arg1: u32, arg2: u32) -> Option<u32> {
        let event = create_ec_decompress_event::<E>(rt, arg1, arg2);
        match E::CURVE_TYPE {
            CurveType::Secp256k1 => rt.record_mut().k256_decompress_events.push(event),
            CurveType::Bls12381 => rt.record_mut().bls12381_decompress_events.push(event),
            _ => panic!("Unsupported curve"),
        }
        None
    }

    fn num_extra_cycles(&self) -> u32 {
        0
    }
}

impl<E: EllipticCurve + WeierstrassParameters> WeierstrassDecompressChip<E> {
    pub const fn new(sign_rule: SignChoiceRule) -> Self {
        Self {
            sign_rule,
            _marker: PhantomData::<E>,
        }
    }

    pub const fn with_lsb_rule() -> Self {
        Self {
            sign_rule: SignChoiceRule::LeastSignificantBit,
            _marker: PhantomData::<E>,
        }
    }

    pub const fn with_lexicographic_rule() -> Self {
        Self {
            sign_rule: SignChoiceRule::Lexicographic,
            _marker: PhantomData::<E>,
        }
    }

    fn populate_field_ops<F: PrimeField32>(
        record: &mut impl ByteRecord,
        shard: u32,
        channel: u32,
        cols: &mut WeierstrassDecompressCols<F, E::BaseField>,
        x: BigUint,
    ) {
        // Y = sqrt(x^3 + b)
        cols.range_x
            .populate(record, shard, channel, &x, &E::BaseField::modulus());
        let x_2 = cols.x_2.populate(
            record,
            shard,
            channel,
            &x.clone(),
            &x.clone(),
            FieldOperation::Mul,
        );
        let x_3 = cols
            .x_3
            .populate(record, shard, channel, &x_2, &x, FieldOperation::Mul);
        let b = E::b_int();
        let x_3_plus_b =
            cols.x_3_plus_b
                .populate(record, shard, channel, &x_3, &b, FieldOperation::Add);

        let sqrt_fn = match E::CURVE_TYPE {
            CurveType::Secp256k1 => secp256k1_sqrt,
            CurveType::Bls12381 => bls12381_sqrt,
            _ => panic!("Unsupported curve"),
        };
        let y = cols
            .y
            .populate(record, shard, channel, &x_3_plus_b, sqrt_fn);

        let zero = BigUint::zero();
        cols.neg_y
            .populate(record, shard, channel, &zero, &y, FieldOperation::Sub);
    }
}

impl<F: PrimeField32, E: EllipticCurve + WeierstrassParameters> MachineAir<F>
    for WeierstrassDecompressChip<E>
{
    type Record = ExecutionRecord;
    type Program = Program;

    fn name(&self) -> String {
        match E::CURVE_TYPE {
            CurveType::Secp256k1 => "Secp256k1Decompress".to_string(),
            CurveType::Bls12381 => "Bls12381Decompress".to_string(),
            _ => panic!("Unsupported curve"),
        }
    }

    fn generate_trace(
        &self,
        input: &ExecutionRecord,
        output: &mut ExecutionRecord,
    ) -> RowMajorMatrix<F> {
        let events = match E::CURVE_TYPE {
            CurveType::Secp256k1 => &input.k256_decompress_events,
            CurveType::Bls12381 => &input.bls12381_decompress_events,
            _ => panic!("Unsupported curve"),
        };

        let mut rows = Vec::new();
        let width = BaseAir::<F>::width(self);

        let mut new_byte_lookup_events = Vec::new();

        for i in 0..events.len() {
            let event = events[i].clone();
            let mut row = vec![F::zero(); width];
            let cols: &mut WeierstrassDecompressCols<F, E::BaseField> =
                row.as_mut_slice().borrow_mut();

            cols.is_real = F::from_bool(true);
            cols.shard = F::from_canonical_u32(event.shard);
            cols.channel = F::from_canonical_u32(event.channel);
            cols.channel = F::from_canonical_u32(event.channel);
            cols.clk = F::from_canonical_u32(event.clk);
            cols.ptr = F::from_canonical_u32(event.ptr);
            cols.is_odd = F::from_canonical_u32(event.is_odd as u32);

            let x = BigUint::from_bytes_le(&event.x_bytes);
            Self::populate_field_ops(
                &mut new_byte_lookup_events,
                event.shard,
                event.channel,
                cols,
                x,
            );

            for i in 0..cols.x_access.len() {
                cols.x_access[i].populate(
                    event.channel,
                    event.x_memory_records[i],
                    &mut new_byte_lookup_events,
                );
            }
            for i in 0..cols.y_access.len() {
                cols.y_access[i].populate_write(
                    event.channel,
                    event.y_memory_records[i],
                    &mut new_byte_lookup_events,
                );
            }

            rows.push(row);
        }
        output.add_byte_lookup_events(new_byte_lookup_events);

        pad_rows(&mut rows, || {
            let mut row = vec![F::zero(); width];
            let cols: &mut WeierstrassDecompressCols<F, E::BaseField> =
                row.as_mut_slice().borrow_mut();

            // take X of the generator as a dummy value to make sure Y^2 = X^3 + b holds
            let dummy_value = E::generator().0;
            let dummy_bytes = dummy_value.to_bytes_le();
            let words = bytes_to_words_le_vec(&dummy_bytes);
            for i in 0..cols.x_access.len() {
                cols.x_access[i].access.value = words[i].into();
            }

            Self::populate_field_ops(&mut vec![], 0, 0, cols, dummy_value);
            row
        });

        let mut trace = RowMajorMatrix::new(rows.into_iter().flatten().collect::<Vec<_>>(), width);

        // Write the nonces to the trace.
        for i in 0..trace.height() {
            let cols: &mut WeierstrassDecompressCols<F, E::BaseField> =
                trace.values[i * width..(i + 1) * width].borrow_mut();
            cols.nonce = F::from_canonical_usize(i);
        }

        trace
    }

    fn included(&self, shard: &Self::Record) -> bool {
        match E::CURVE_TYPE {
            CurveType::Secp256k1 => !shard.k256_decompress_events.is_empty(),
            CurveType::Bls12381 => !shard.bls12381_decompress_events.is_empty(),
            _ => panic!("Unsupported curve"),
        }
    }
}

impl<F, E: EllipticCurve> BaseAir<F> for WeierstrassDecompressChip<E> {
    fn width(&self) -> usize {
        num_weierstrass_decompress_cols::<E::BaseField>()
            + match self.sign_rule {
                SignChoiceRule::LeastSignificantBit => 0,
                SignChoiceRule::Lexicographic => size_of::<FieldLtCols<u8, E::BaseField>>(),
            }
    }
}

impl<AB, E: EllipticCurve + WeierstrassParameters> Air<AB> for WeierstrassDecompressChip<E>
where
    AB: SP1AirBuilder,
    Limbs<AB::Var, <E::BaseField as NumLimbs>::Limbs>: Copy,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();

        let weierstrass_cols = num_weierstrass_decompress_cols::<E::BaseField>();
        let local_slice = main.row_slice(0);
        let local: &WeierstrassDecompressCols<AB::Var, E::BaseField> =
            (*local_slice)[0..weierstrass_cols].borrow();
        let next = main.row_slice(1);
        let next: &WeierstrassDecompressCols<AB::Var, E::BaseField> =
            (*next)[0..weierstrass_cols].borrow();

        // Constrain the incrementing nonce.
        builder.when_first_row().assert_zero(local.nonce);
        builder
            .when_transition()
            .assert_eq(local.nonce + AB::Expr::one(), next.nonce);

        let num_limbs = <E::BaseField as NumLimbs>::Limbs::USIZE;
        let num_words_field_element = num_limbs / 4;

        builder.assert_bool(local.is_odd);

        let x: Limbs<AB::Var, <E::BaseField as NumLimbs>::Limbs> =
            limbs_from_prev_access(&local.x_access);
        let max_num_limbs = E::BaseField::to_limbs_field_vec(&E::BaseField::modulus());
        local.range_x.eval(
            builder,
            &x,
            &limbs_from_vec::<AB::Expr, <E::BaseField as NumLimbs>::Limbs, AB::F>(max_num_limbs),
            local.shard,
            local.channel,
            local.is_real,
        );
        local.x_2.eval(
            builder,
            &x,
            &x,
            FieldOperation::Mul,
            local.shard,
            local.channel,
            local.is_real,
        );
        local.x_3.eval(
            builder,
            &local.x_2.result,
            &x,
            FieldOperation::Mul,
            local.shard,
            local.channel,
            local.is_real,
        );
        let b = E::b_int();
        let b_const = E::BaseField::to_limbs_field::<AB::F, _>(&b);
        local.x_3_plus_b.eval(
            builder,
            &local.x_3.result,
            &b_const,
            FieldOperation::Add,
            local.shard,
            local.channel,
            local.is_real,
        );

        local.neg_y.eval(
            builder,
            &[AB::Expr::zero()].iter(),
            &local.y.multiplication.result,
            FieldOperation::Sub,
            local.shard,
            local.channel,
            local.is_real,
        );

        local.y.eval(
            builder,
            &local.x_3_plus_b.result,
            local.y.lsb,
            local.shard,
            local.channel,
            local.is_real,
        );

        let y_limbs: Limbs<AB::Var, <E::BaseField as NumLimbs>::Limbs> =
            limbs_from_access(&local.y_access);

        match self.sign_rule {
            SignChoiceRule::LeastSignificantBit => {
                builder
                    .when(local.is_real)
                    .when_ne(local.y.lsb, AB::Expr::one() - local.is_odd)
                    .assert_all_eq(local.y.multiplication.result, y_limbs);
                builder
                    .when(local.is_real)
                    .when_ne(local.y.lsb, local.is_odd)
                    .assert_all_eq(local.neg_y.result, y_limbs);
            }
            SignChoiceRule::Lexicographic => {
                let lt_cols: &FieldLtCols<AB::Var, E::BaseField> = (*local_slice)[weierstrass_cols
                    ..weierstrass_cols + size_of::<FieldLtCols<u8, E::BaseField>>()]
                    .borrow();
            }
        }

        for i in 0..num_words_field_element {
            builder.eval_memory_access(
                local.shard,
                local.channel,
                local.clk,
                local.ptr.into() + AB::F::from_canonical_u32((i as u32) * 4 + num_limbs as u32),
                &local.x_access[i],
                local.is_real,
            );
        }
        for i in 0..num_words_field_element {
            builder.eval_memory_access(
                local.shard,
                local.channel,
                local.clk,
                local.ptr.into() + AB::F::from_canonical_u32((i as u32) * 4),
                &local.y_access[i],
                local.is_real,
            );
        }

        let syscall_id = match E::CURVE_TYPE {
            CurveType::Secp256k1 => {
                AB::F::from_canonical_u32(SyscallCode::SECP256K1_DECOMPRESS.syscall_id())
            }
            CurveType::Bls12381 => {
                AB::F::from_canonical_u32(SyscallCode::BLS12381_DECOMPRESS.syscall_id())
            }
            _ => panic!("Unsupported curve"),
        };

        builder.receive_syscall(
            local.shard,
            local.channel,
            local.clk,
            local.nonce,
            syscall_id,
            local.ptr,
            local.is_odd,
            local.is_real,
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::io::SP1Stdin;
    use crate::stark::DefaultProver;
    use crate::utils::{self, tests::BLS12381_DECOMPRESS_ELF};
    use crate::Program;
    use amcl::bls381::bls381::basic::key_pair_generate_g2;
    use amcl::bls381::bls381::utils::deserialize_g1;
    use amcl::rand::RAND;
    use elliptic_curve::sec1::ToEncodedPoint;
    use rand::{thread_rng, Rng};

    use crate::utils::run_test_io;
    use crate::utils::tests::SECP256K1_DECOMPRESS_ELF;

    #[test]
    fn test_weierstrass_bls_decompress() {
        utils::setup_logger();
        let mut rng = thread_rng();
        let mut rand = RAND::new();

        let len = 100;
        let num_tests = 1;
        let random_slice = (0..len).map(|_| rng.gen::<u8>()).collect::<Vec<u8>>();
        rand.seed(len, &random_slice);

        for _ in 0..num_tests {
            let (_, compressed) = key_pair_generate_g2(&mut rand);
            // let compressed = hex::decode("8dffed32f74d62cf8904a02fc7f564a224938c2571f138acd059c0d2f10914e77a1528b1616f77ff5d28079b88d8da8d").unwrap();

            let stdin = SP1Stdin::from(&compressed);
            let mut public_values =
                run_test_io::<DefaultProver<_, _>>(Program::from(BLS12381_DECOMPRESS_ELF), stdin)
                    .unwrap();

            let mut result = [0; 96];
            public_values.read_slice(&mut result);

            let point = deserialize_g1(&compressed).unwrap();
            let x = point.getx().to_string();
            let y = point.gety().to_string();
            let decompressed = hex::decode(format!("{x}{y}")).unwrap();
            assert_eq!(result, decompressed.as_slice());
        }
    }

    #[test]
    fn test_weierstrass_k256_decompress() {
        utils::setup_logger();

        let mut rng = thread_rng();

        // TODO: Change back to 10 after debugging.
        let num_tests = 1;

        for _ in 0..num_tests {
            let secret_key = k256::SecretKey::random(&mut rng);
            let public_key = secret_key.public_key();
            let encoded = public_key.to_encoded_point(false);
            let decompressed = encoded.as_bytes();
            let compressed = public_key.to_sec1_bytes();

            let inputs = SP1Stdin::from(&compressed);

            let mut public_values =
                run_test_io::<DefaultProver<_, _>>(Program::from(SECP256K1_DECOMPRESS_ELF), inputs)
                    .unwrap();
            let mut result = [0; 65];
            public_values.read_slice(&mut result);
            assert_eq!(result, decompressed);
        }
    }
}
