use crate::air::{BaseAirBuilder, MachineAir, Polynomial, SP1AirBuilder};
use crate::bytes::event::ByteRecord;
use crate::bytes::ByteLookupEvent;
use crate::memory::{value_as_limbs, MemoryReadCols, MemoryWriteCols};
use crate::operations::field::field_op::{FieldOpCols, FieldOperation};
use crate::operations::field::params::NumWords;
use crate::operations::field::params::{Limbs, NumLimbs};
use crate::runtime::{ExecutionRecord, Program, Syscall, SyscallCode, SyscallContext};
use crate::runtime::{MemoryReadRecord, MemoryWriteRecord};
use crate::utils::{limbs_from_prev_access, pad_rows, words_to_bytes_le_vec};
use generic_array::GenericArray;
use itertools::Itertools;
use num::BigUint;
use num::Zero;
use p3_air::AirBuilder;
use p3_air::{Air, BaseAir};
use p3_field::AbstractField;
use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;
use serde::{Deserialize, Serialize};
use sp1_derive::AlignedBorrow;
use std::borrow::{Borrow, BorrowMut};
use std::marker::PhantomData;
use std::mem::size_of;
use typenum::Unsigned;

use super::{FieldType, FpOpField};

pub const fn num_fp2_addsub_cols<P: FpOpField>() -> usize {
    size_of::<Fp2AddSubAssignCols<u8, P>>()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fp2AddSubEvent {
    pub lookup_id: usize,
    pub shard: u32,
    pub channel: u8,
    pub clk: u32,
    pub op: FieldOperation,
    pub x_ptr: u32,
    pub x: Vec<u32>,
    pub y_ptr: u32,
    pub y: Vec<u32>,
    pub x_memory_records: Vec<MemoryWriteRecord>,
    pub y_memory_records: Vec<MemoryReadRecord>,
}

/// A set of columns for the Fp2AddSub operation.
#[derive(Debug, Clone, AlignedBorrow)]
#[repr(C)]
pub struct Fp2AddSubAssignCols<T, P: FpOpField> {
    pub is_real: T,
    pub shard: T,
    pub channel: T,
    pub nonce: T,
    pub clk: T,
    pub x_ptr: T,
    pub y_ptr: T,
    pub x_access: GenericArray<MemoryWriteCols<T>, P::WordsCurvePoint>,
    pub y_access: GenericArray<MemoryReadCols<T>, P::WordsCurvePoint>,
    pub(crate) c0: FieldOpCols<T, P>,
    pub(crate) c1: FieldOpCols<T, P>,
}

pub struct Fp2AddSubAssignChip<P> {
    _marker: PhantomData<P>,
    op: FieldOperation,
}

impl<P: FpOpField> Syscall for Fp2AddSubAssignChip<P> {
    fn execute(&self, rt: &mut SyscallContext, arg1: u32, arg2: u32) -> Option<u32> {
        let clk = rt.clk;
        let x_ptr = arg1;
        if x_ptr % 4 != 0 {
            panic!();
        }
        let y_ptr = arg2;
        if y_ptr % 4 != 0 {
            panic!();
        }

        let num_words = <P as NumWords>::WordsCurvePoint::USIZE;

        let x = rt.slice_unsafe(x_ptr, num_words);
        let (y_memory_records, y) = rt.mr_slice(y_ptr, num_words);
        rt.clk += 1;

        let (ac0, ac1) = x.split_at(x.len() / 2);
        let (bc0, bc1) = y.split_at(y.len() / 2);

        let ac0 = &BigUint::from_slice(ac0);
        let ac1 = &BigUint::from_slice(ac1);
        let bc0 = &BigUint::from_slice(bc0);
        let bc1 = &BigUint::from_slice(bc1);
        let modulus = &BigUint::from_bytes_le(P::MODULUS);

        let (c0, c1) = match self.op {
            FieldOperation::Add => ((ac0 + bc0) % modulus, (ac1 + bc1) % modulus),
            FieldOperation::Sub => (
                (ac0 + modulus - bc0) % modulus,
                (ac1 + modulus - bc1) % modulus,
            ),
            _ => panic!("Invalid operation"),
        };

        let mut result = c0
            .to_u32_digits()
            .into_iter()
            .chain(c1.to_u32_digits())
            .collect::<Vec<u32>>();

        result.resize(num_words, 0);
        let x_memory_records = rt.mw_slice(x_ptr, &result);

        let lookup_id = rt.syscall_lookup_id as usize;
        let shard = rt.current_shard();
        let channel = rt.current_channel();
        let op = self.op;
        match P::FIELD_TYPE {
            FieldType::Bn254 => {
                rt.record_mut()
                    .bn254_fp2_addsub_events
                    .push(Fp2AddSubEvent {
                        lookup_id,
                        shard,
                        channel,
                        clk,
                        op,
                        x_ptr,
                        x,
                        y_ptr,
                        y,
                        x_memory_records,
                        y_memory_records,
                    });
            }
            FieldType::Bls12381 => {
                rt.record_mut()
                    .bls12381_fp2_addsub_events
                    .push(Fp2AddSubEvent {
                        lookup_id,
                        shard,
                        channel,
                        clk,
                        op,
                        x_ptr,
                        x,
                        y_ptr,
                        y,
                        x_memory_records,
                        y_memory_records,
                    });
            }
        }
        None
    }

    fn num_extra_cycles(&self) -> u32 {
        1
    }
}

impl<P: FpOpField> Fp2AddSubAssignChip<P> {
    pub const fn new(op: FieldOperation) -> Self {
        Self {
            _marker: PhantomData,
            op,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn populate_field_ops<F: PrimeField32>(
        blu_events: &mut Vec<ByteLookupEvent>,
        shard: u32,
        channel: u8,
        cols: &mut Fp2AddSubAssignCols<F, P>,
        p_x: BigUint,
        p_y: BigUint,
        q_x: BigUint,
        q_y: BigUint,
        op: FieldOperation,
    ) {
        let modulus_bytes = P::MODULUS;
        let modulus = BigUint::from_bytes_le(modulus_bytes);
        cols.c0
            .populate_with_modulus(blu_events, shard, channel, &p_x, &q_x, &modulus, op);
        cols.c1
            .populate_with_modulus(blu_events, shard, channel, &p_y, &q_y, &modulus, op);
    }
}

impl<F: PrimeField32, P: FpOpField> MachineAir<F> for Fp2AddSubAssignChip<P> {
    type Record = ExecutionRecord;

    type Program = Program;

    fn name(&self) -> String {
        let op = match self.op {
            FieldOperation::Add => "Add".to_string(),
            FieldOperation::Sub => "Sub".to_string(),
            _ => unreachable!("Invalid operation"),
        };
        match P::FIELD_TYPE {
            FieldType::Bn254 => format!("Bn254Fp2{}Assign", op),
            FieldType::Bls12381 => format!("Bls12831Fp2{}Assign", op),
        }
    }

    fn generate_trace(&self, input: &Self::Record, output: &mut Self::Record) -> RowMajorMatrix<F> {
        let events = match P::FIELD_TYPE {
            FieldType::Bn254 => &input.bn254_fp2_addsub_events,
            FieldType::Bls12381 => &input.bls12381_fp2_addsub_events,
        };

        let mut rows = Vec::new();
        let mut new_byte_lookup_events = Vec::new();

        for i in 0..events.len() {
            let event = &events[i];
            if event.op != self.op {
                continue;
            }
            let mut row = vec![F::zero(); num_fp2_addsub_cols::<P>()];
            let cols: &mut Fp2AddSubAssignCols<F, P> = row.as_mut_slice().borrow_mut();

            let p = &event.x;
            let q = &event.y;
            let p_x = BigUint::from_bytes_le(&words_to_bytes_le_vec(&p[..p.len() / 2]));
            let p_y = BigUint::from_bytes_le(&words_to_bytes_le_vec(&p[p.len() / 2..]));
            let q_x = BigUint::from_bytes_le(&words_to_bytes_le_vec(&q[..q.len() / 2]));
            let q_y = BigUint::from_bytes_le(&words_to_bytes_le_vec(&q[q.len() / 2..]));

            cols.is_real = F::one();
            cols.shard = F::from_canonical_u32(event.shard);
            cols.channel = F::from_canonical_u8(event.channel);
            cols.clk = F::from_canonical_u32(event.clk);
            cols.x_ptr = F::from_canonical_u32(event.x_ptr);
            cols.y_ptr = F::from_canonical_u32(event.y_ptr);

            Self::populate_field_ops(
                &mut new_byte_lookup_events,
                event.shard,
                event.channel,
                cols,
                p_x,
                p_y,
                q_x,
                q_y,
                self.op,
            );

            // Populate the memory access columns.
            for i in 0..cols.y_access.len() {
                cols.y_access[i].populate(
                    event.channel,
                    event.y_memory_records[i],
                    &mut new_byte_lookup_events,
                );
            }
            for i in 0..cols.x_access.len() {
                cols.x_access[i].populate(
                    event.channel,
                    event.x_memory_records[i],
                    &mut new_byte_lookup_events,
                );
            }
            rows.push(row)
        }

        output.add_byte_lookup_events(new_byte_lookup_events);

        pad_rows(&mut rows, || {
            let mut row = vec![F::zero(); num_fp2_addsub_cols::<P>()];
            let cols: &mut Fp2AddSubAssignCols<F, P> = row.as_mut_slice().borrow_mut();
            let zero = BigUint::zero();
            Self::populate_field_ops(
                &mut vec![],
                0,
                0,
                cols,
                zero.clone(),
                zero.clone(),
                zero.clone(),
                zero,
                self.op,
            );
            row
        });

        // Convert the trace to a row major matrix.
        let mut trace = RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            num_fp2_addsub_cols::<P>(),
        );

        // Write the nonces to the trace.
        for i in 0..trace.height() {
            let cols: &mut Fp2AddSubAssignCols<F, P> = trace.values
                [i * num_fp2_addsub_cols::<P>()..(i + 1) * num_fp2_addsub_cols::<P>()]
                .borrow_mut();
            cols.nonce = F::from_canonical_usize(i);
        }

        trace
    }

    fn included(&self, shard: &Self::Record) -> bool {
        match P::FIELD_TYPE {
            FieldType::Bn254 => !shard.bn254_fp2_addsub_events.is_empty(),
            FieldType::Bls12381 => !shard.bls12381_fp2_addsub_events.is_empty(),
        }
    }
}

impl<F, P: FpOpField> BaseAir<F> for Fp2AddSubAssignChip<P> {
    fn width(&self) -> usize {
        num_fp2_addsub_cols::<P>()
    }
}

impl<AB, P: FpOpField> Air<AB> for Fp2AddSubAssignChip<P>
where
    AB: SP1AirBuilder,
    Limbs<AB::Var, <P as NumLimbs>::Limbs>: Copy,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local = main.row_slice(0);
        let local: &Fp2AddSubAssignCols<AB::Var, P> = (*local).borrow();
        let next = main.row_slice(1);
        let next: &Fp2AddSubAssignCols<AB::Var, P> = (*next).borrow();

        builder.when_first_row().assert_zero(local.nonce);
        builder
            .when_transition()
            .assert_eq(local.nonce + AB::Expr::one(), next.nonce);
        let num_words_field_element = <P as NumLimbs>::Limbs::USIZE / 4;

        let p_x = limbs_from_prev_access(&local.x_access[0..num_words_field_element]);
        let p_y = limbs_from_prev_access(&local.x_access[num_words_field_element..]);

        let q_x = limbs_from_prev_access(&local.y_access[0..num_words_field_element]);
        let q_y = limbs_from_prev_access(&local.y_access[num_words_field_element..]);

        let modulus_coeffs = P::MODULUS
            .iter()
            .map(|&limbs| AB::Expr::from_canonical_u8(limbs))
            .collect_vec();
        let p_modulus = Polynomial::from_coefficients(&modulus_coeffs);

        {
            local.c0.eval_with_modulus(
                builder,
                &p_x,
                &q_x,
                &p_modulus,
                self.op,
                local.shard,
                local.channel,
                local.is_real,
            );

            local.c1.eval_with_modulus(
                builder,
                &p_y,
                &q_y,
                &p_modulus,
                self.op,
                local.shard,
                local.channel,
                local.is_real,
            );
        }

        builder.when(local.is_real).assert_all_eq(
            local.c0.result,
            value_as_limbs(&local.x_access[0..num_words_field_element]),
        );
        builder.when(local.is_real).assert_all_eq(
            local.c1.result,
            value_as_limbs(&local.x_access[num_words_field_element..]),
        );
        builder.eval_memory_access_slice(
            local.shard,
            local.channel,
            local.clk.into(),
            local.y_ptr,
            &local.y_access,
            local.is_real,
        );
        builder.eval_memory_access_slice(
            local.shard,
            local.channel,
            local.clk + AB::F::from_canonical_u32(1), // We read p at +1 since p, q could be the same.
            local.x_ptr,
            &local.x_access,
            local.is_real,
        );

        let syscall_id_felt = match P::FIELD_TYPE {
            FieldType::Bn254 => match self.op {
                FieldOperation::Add => {
                    AB::F::from_canonical_u32(SyscallCode::BN254_FP2_ADD.syscall_id())
                }
                FieldOperation::Sub => {
                    AB::F::from_canonical_u32(SyscallCode::BN254_FP2_SUB.syscall_id())
                }
                _ => panic!("Invalid operation"),
            },
            FieldType::Bls12381 => match self.op {
                FieldOperation::Add => {
                    AB::F::from_canonical_u32(SyscallCode::BLS12381_FP2_ADD.syscall_id())
                }
                FieldOperation::Sub => {
                    AB::F::from_canonical_u32(SyscallCode::BLS12381_FP2_SUB.syscall_id())
                }
                _ => panic!("Invalid operation"),
            },
        };

        builder.receive_syscall(
            local.shard,
            local.channel,
            local.clk,
            local.nonce,
            syscall_id_felt,
            local.x_ptr,
            local.y_ptr,
            local.is_real,
        );
    }
}
