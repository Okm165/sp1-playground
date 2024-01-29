use crate::air::CurtaAirBuilder;
use crate::cpu::columns::MemoryAccessCols;
use crate::cpu::MemoryReadRecord;
use crate::cpu::MemoryWriteRecord;
use crate::operations::field::fp_den::FpDenCols;
use crate::operations::field::fp_inner_product::FpInnerProductCols;
use crate::operations::field::fp_op::FpOpCols;
use crate::operations::field::fp_op::FpOperation;
use crate::operations::field::params::Limbs;
use crate::operations::field::params::NUM_LIMBS;
use crate::precompiles::PrecompileRuntime;
use crate::runtime::Segment;
use crate::utils::ec::add::create_elliptic_curve_add_event;
use crate::utils::ec::field::FieldParameters;
use crate::utils::ec::weierstrass::WeierstrassParameters;
use crate::utils::ec::AffinePoint;
use crate::utils::ec::EllipticCurve;
use crate::utils::limbs_from_prev_access;
use crate::utils::pad_rows;
use crate::utils::Chip;
use core::borrow::{Borrow, BorrowMut};
use core::mem::size_of;
use num::BigUint;
use num::Zero;
use p3_air::AirBuilder;
use p3_air::{Air, BaseAir};
use p3_field::AbstractField;
use p3_field::Field;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::MatrixRowSlices;
use std::fmt::Debug;
use std::marker::PhantomData;
use valida_derive::AlignedBorrow;

#[derive(Debug, Clone, Copy)]
pub struct WeierstrassAddEvent {
    pub clk: u32,
    pub p_ptr: u32,
    pub p: [u32; 16],
    pub q_ptr: u32,
    pub q: [u32; 16],
    pub q_ptr_record: MemoryReadRecord,
    pub p_memory_records: [MemoryWriteRecord; 16],
    pub q_memory_records: [MemoryReadRecord; 16],
}

pub const NUM_ED_ADD_COLS: usize = size_of::<WeierstrassAddAssignCols<u8>>();

/// A set of columns to compute `WeierstrassAdd` where a, b are field elements.
/// Right now the number of limbs is assumed to be a constant, although this could be macro-ed
/// or made generic in the future.
#[derive(Debug, Clone, AlignedBorrow)]
#[repr(C)]
pub struct WeierstrassAddAssignCols<T> {
    pub is_real: T,
    pub segment: T,
    pub clk: T,
    pub p_ptr: T,
    pub q_ptr: T,
    pub q_ptr_access: MemoryAccessCols<T>,
    pub p_access: [MemoryAccessCols<T>; 16],
    pub q_access: [MemoryAccessCols<T>; 16],
    pub(crate) q_x_minus_p_x: FpOpCols<T>,
    pub(crate) lambda_numerator: FpOpCols<T>,
    pub(crate) lambda: FpDenCols<T>,
    pub(crate) lambda_squared: FpOpCols<T>,
    pub(crate) x_3_ins: FpOpCols<T>,
    pub(crate) y_3_ins: FpOpCols<T>,
    pub(crate) p_x_minus_x: FpOpCols<T>,
    pub(crate) lambda_times_p_x_minus_x: FpOpCols<T>,
}

pub struct WeierstrassAddAssignChip<E, WP> {
    _marker: PhantomData<(E, WP)>,
}

impl<E: EllipticCurve, WP: WeierstrassParameters> WeierstrassAddAssignChip<E, WP> {
    pub const NUM_CYCLES: u32 = 8;

    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
    pub fn execute(rt: &mut PrecompileRuntime) -> u32 {
        let event = create_elliptic_curve_add_event::<E>(rt);
        rt.segment_mut().weierstrass_add_events.push(event);
        event.p_ptr + 1
    }

    fn populate_fp_ops<F: Field>(
        cols: &mut WeierstrassAddAssignCols<F>,
        p_x: BigUint,
        p_y: BigUint,
        q_x: BigUint,
        q_y: BigUint,
    ) {
        // This copied & pasted code can help me figure out the syntax, but the logic is likely
        // completely different.
        // let x3_numerator = cols
        //     .x3_numerator
        //     .populate::<E::BaseField>(&[p_x.clone(), q_x.clone()], &[q_y.clone(), p_y.clone()]);

        // q_x - p_x is used in multiple places.
        let q_x_minus_p_x =
            cols.q_x_minus_p_x
                .populate::<E::BaseField>(&q_x, &p_x, FpOperation::Sub);

        // lambda = (q_y - p_y) / (q_x - p_x)
        let lambda = {
            let lambda_numerator =
                cols.lambda_numerator
                    .populate::<E::BaseField>(&q_y, &p_y, FpOperation::Sub);
            cols.lambda
                .populate::<E::BaseField>(&lambda_numerator, &q_x_minus_p_x, false)
        };

        // x = lambda^2 - p_x - q_x
        let x = {
            let lambda_squared =
                cols.lambda_squared
                    .populate::<E::BaseField>(&lambda, &lambda, FpOperation::Mul);
            cols.x_3_ins
                .populate::<E::BaseField>(&lambda_squared, &p_x, FpOperation::Sub)
        };

        // y = lambda * (p_x - x) - p_y
        {
            let p_x_minus_x = cols
                .p_x_minus_x
                .populate::<E::BaseField>(&p_x, &x, FpOperation::Sub);
            let lambda_times_p_x_minus_x = cols.lambda_times_p_x_minus_x.populate::<E::BaseField>(
                &lambda,
                &p_x_minus_x,
                FpOperation::Mul,
            );
            cols.y_3_ins.populate::<E::BaseField>(
                &lambda_times_p_x_minus_x,
                &p_y,
                FpOperation::Sub,
            );
        }
    }
}

impl<F: Field, E: EllipticCurve, WP: WeierstrassParameters> Chip<F>
    for WeierstrassAddAssignChip<E, WP>
{
    fn name(&self) -> String {
        "WeierstrassAddAssign".to_string()
    }

    fn generate_trace(&self, segment: &mut Segment) -> RowMajorMatrix<F> {
        // This is wrong.
        let mut rows = Vec::new();

        let mut new_field_events = Vec::new();

        for i in 0..segment.ed_add_events.len() {
            let event = segment.ed_add_events[i];
            let mut row = [F::zero(); NUM_ED_ADD_COLS];
            let cols: &mut WeierstrassAddAssignCols<F> = unsafe { std::mem::transmute(&mut row) };

            // Decode affine points.
            let p = &event.p;
            let q = &event.q;
            let p = AffinePoint::<E>::from_words_le(p);
            let (p_x, p_y) = (p.x, p.y);
            let q = AffinePoint::<E>::from_words_le(q);
            let (q_x, q_y) = (q.x, q.y);

            // Populate basic columns.
            cols.is_real = F::one();
            cols.segment = F::from_canonical_u32(segment.index);
            cols.clk = F::from_canonical_u32(event.clk);
            cols.p_ptr = F::from_canonical_u32(event.p_ptr);
            cols.q_ptr = F::from_canonical_u32(event.q_ptr);

            Self::populate_fp_ops(cols, p_x, p_y, q_x, q_y);

            // Populate the memory access columns.
            for i in 0..16 {
                cols.q_access[i].populate_read(event.q_memory_records[i], &mut new_field_events);
            }
            for i in 0..16 {
                cols.p_access[i].populate_write(event.p_memory_records[i], &mut new_field_events);
            }
            cols.q_ptr_access
                .populate_read(event.q_ptr_record, &mut new_field_events);

            rows.push(row);
        }
        segment.field_events.extend(new_field_events);

        pad_rows(&mut rows, || {
            let mut row = [F::zero(); NUM_ED_ADD_COLS];
            let cols: &mut WeierstrassAddAssignCols<F> = unsafe { std::mem::transmute(&mut row) };
            let zero = BigUint::zero();
            Self::populate_fp_ops(cols, zero.clone(), zero.clone(), zero.clone(), zero);
            row
        });

        // Convert the trace to a row major matrix.
        RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_ED_ADD_COLS,
        )
    }
}

impl<F, E: EllipticCurve, WP: WeierstrassParameters> BaseAir<F>
    for WeierstrassAddAssignChip<E, WP>
{
    fn width(&self) -> usize {
        NUM_ED_ADD_COLS
    }
}

impl<AB, E: EllipticCurve, WP: WeierstrassParameters> Air<AB> for WeierstrassAddAssignChip<E, WP>
where
    AB: CurtaAirBuilder,
{
    fn eval(&self, builder: &mut AB) {
        // TODO: This is wrong.
        let main = builder.main();
        let row: &WeierstrassAddAssignCols<AB::Var> = main.row_slice(0).borrow();

        let x1 = limbs_from_prev_access(&row.p_access[0..8]);
        let x2 = limbs_from_prev_access(&row.q_access[0..8]);
        let y1 = limbs_from_prev_access(&row.p_access[8..16]);
        let y2 = limbs_from_prev_access(&row.q_access[8..16]);

        // TODO: This FP stuff is likely irrelevant.
        // // x3_numerator = x1 * y2 + x2 * y1.
        // row.x3_numerator
        //     .eval::<AB, E::BaseField>(builder, &[x1, x2], &[y2, y1]);

        // // y3_numerator = y1 * y2 + x1 * x2.
        // row.y3_numerator
        //     .eval::<AB, E::BaseField>(builder, &[y1, x1], &[y2, x2]);

        // // f = x1 * x2 * y1 * y2.
        // row.x1_mul_y1
        //     .eval::<AB, E::BaseField, _, _>(builder, &x1, &y1, FpOperation::Mul);
        // row.x2_mul_y2
        //     .eval::<AB, E::BaseField, _, _>(builder, &x2, &y2, FpOperation::Mul);

        // let x1_mul_y1 = row.x1_mul_y1.result;
        // let x2_mul_y2 = row.x2_mul_y2.result;
        // row.f
        //     .eval::<AB, E::BaseField, _, _>(builder, &x1_mul_y1, &x2_mul_y2, FpOperation::Mul);

        // // d * f.
        // let f = row.f.result;
        // let d_biguint = WP::d_biguint();
        // let d_const = E::BaseField::to_limbs_field::<AB::F>(&d_biguint);
        // let d_const_expr = Limbs::<AB::Expr>(d_const.0.map(|x| x.into()));
        // row.d_mul_f
        //     .eval::<AB, E::BaseField, _, _>(builder, &f, &d_const_expr, FpOperation::Mul);

        // let d_mul_f = row.d_mul_f.result;

        // // x3 = x3_numerator / (1 + d * f).
        // row.x3_ins
        //     .eval::<AB, E::BaseField>(builder, &row.x3_numerator.result, &d_mul_f, true);

        // // y3 = y3_numerator / (1 - d * f).
        // row.y3_ins
        //     .eval::<AB, E::BaseField>(builder, &row.y3_numerator.result, &d_mul_f, false);

        // Constraint self.p_access.value = [self.x3_ins.result, self.y3_ins.result]
        // This is to ensure that p_access is updated with the new value.
        for i in 0..NUM_LIMBS {
            builder
                .when(row.is_real)
                .assert_eq(row.x3_ins.result[i], row.p_access[i / 4].value[i % 4]);
            builder
                .when(row.is_real)
                .assert_eq(row.y3_ins.result[i], row.p_access[8 + i / 4].value[i % 4]);
        }

        builder.constraint_memory_access(
            row.segment,
            row.clk, // clk + 0 -> C
            AB::F::from_canonical_u32(11),
            row.q_ptr_access,
            row.is_real,
        );
        for i in 0..16 {
            builder.constraint_memory_access(
                row.segment,
                row.clk, // clk + 0 -> Memory
                row.q_ptr + AB::F::from_canonical_u32(i * 4),
                row.q_access[i as usize],
                row.is_real,
            );
        }
        for i in 0..16 {
            builder.constraint_memory_access(
                row.segment,
                row.clk + AB::F::from_canonical_u32(4), // clk + 4 -> Memory
                row.p_ptr + AB::F::from_canonical_u32(i * 4),
                row.p_access[i as usize],
                row.is_real,
            );
        }
    }
}

#[cfg(test)]
pub mod tests {

    use crate::{
        runtime::Program,
        utils::{prove, setup_logger},
    };

    #[test]
    fn test_weierstrass_add_simple() {
        setup_logger();
        // TODO: This file doesn't exist.
        let program = Program::from_elf("../programs/weirstrass_add");
        prove(program);
    }
}
