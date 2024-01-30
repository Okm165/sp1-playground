use crate::air::CurtaAirBuilder;
use crate::cpu::columns::MemoryAccessCols;
use crate::cpu::MemoryReadRecord;
use crate::cpu::MemoryWriteRecord;
use crate::operations::field::fp_op::FpOpCols;
use crate::operations::field::fp_op::FpOperation;
use crate::operations::field::params::NUM_LIMBS;
use crate::precompiles::create_ec_add_event;
use crate::precompiles::PrecompileRuntime;
use crate::runtime::Segment;
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

pub const NUM_WEIERSTRASS_ADD_COLS: usize = size_of::<WeierstrassAddAssignCols<u8>>();

/// A set of columns to compute `WeierstrassAdd` where a, b are field elements.
///
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
    pub(crate) slope_denominator: FpOpCols<T>,
    pub(crate) slope_numerator: FpOpCols<T>,
    pub(crate) slope: FpOpCols<T>,
    pub(crate) slope_squared: FpOpCols<T>,
    pub(crate) p_x_plus_q_x: FpOpCols<T>,
    pub(crate) x3_ins: FpOpCols<T>,
    pub(crate) p_x_minus_x: FpOpCols<T>,
    pub(crate) y3_ins: FpOpCols<T>,
    pub(crate) slope_times_p_x_minus_x: FpOpCols<T>,
}

pub struct WeierstrassAddAssignChip<E, WP> {
    _marker: PhantomData<(E, WP)>,
}

impl<E: EllipticCurve, WP: WeierstrassParameters> WeierstrassAddAssignChip<E, WP> {
    pub const NUM_CYCLES: u32 = 8;

    pub fn new() -> Self {
        println!("WeierstrassAddAssignChip::new");
        Self {
            _marker: PhantomData,
        }
    }

    pub fn execute(rt: &mut PrecompileRuntime) -> u32 {
        println!("WeierstrassAddAssignChip::execute");
        let event = create_ec_add_event::<E>(rt);
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
        // This populates necessary field operations to calculate the addition of two points on a
        // Weierstrass curve.

        // These print out 0's for the padded rows as expected.
        println!("p_x = {}", p_x);
        println!("p_y = {}", p_y);
        println!("q_x = {}", q_x);
        println!("q_y = {}", q_y);

        // Commented out everything except this for debugging.
        let slope_numerator =
            cols.slope_numerator
                .populate::<E::BaseField>(&q_y, &p_y, FpOperation::Sub);

        // // Slope = (q.y - p.y) / (q.x - p.x).
        // let slope = {
        //     let slope_denominator =
        //         cols.slope_denominator
        //             .populate::<E::BaseField>(&q_x, &p_x, FpOperation::Sub);

        //     cols.slope.populate::<E::BaseField>(
        //         &slope_numerator,
        //         &slope_denominator,
        //         FpOperation::Div,
        //     )
        // };

        // // x = slope * slope - (p.x + q.x)
        // let x = {
        //     let slope_squared =
        //         cols.slope_squared
        //             .populate::<E::BaseField>(&slope, &slope, FpOperation::Mul);
        //     let p_x_plus_q_x =
        //         cols.p_x_plus_q_x
        //             .populate::<E::BaseField>(&p_x, &q_x, FpOperation::Add);
        //     cols.x3_ins
        //         .populate::<E::BaseField>(&slope_squared, &p_x_plus_q_x, FpOperation::Sub)
        // };

        // // y = slope * (p.x - x_3n) - p.y
        // let y = {
        //     let p_x_minus_x = cols
        //         .p_x_minus_x
        //         .populate::<E::BaseField>(&p_x, &x, FpOperation::Sub);
        //     let slope_times_p_x_minus_x = cols.slope_times_p_x_minus_x.populate::<E::BaseField>(
        //         &slope,
        //         &p_x_minus_x,
        //         FpOperation::Mul,
        //     );
        //     cols.y3_ins
        //         .populate::<E::BaseField>(&slope_times_p_x_minus_x, &p_y, FpOperation::Sub)
        // };
        // println!("added result x = {}", x);
        // println!("added result y = {}", y);
    }
}

impl<F: Field, E: EllipticCurve, WP: WeierstrassParameters> Chip<F>
    for WeierstrassAddAssignChip<E, WP>
{
    fn name(&self) -> String {
        "WeierstrassAddAssign".to_string()
    }

    fn generate_trace(&self, segment: &mut Segment) -> RowMajorMatrix<F> {
        // This has been copied and pasted from ed_add.rs and I updated this so this is for
        // Weierstrass curves.
        let mut rows = Vec::new();

        let mut new_field_events = Vec::new();

        for i in 0..segment.weierstrass_add_events.len() {
            let event = segment.weierstrass_add_events[i];
            let mut row = [F::zero(); NUM_WEIERSTRASS_ADD_COLS];
            let cols: &mut WeierstrassAddAssignCols<F> = unsafe { std::mem::transmute(&mut row) };

            // Decode affine points.
            let p = &event.p;
            let q = &event.q;
            println!("p = {:?}", p);
            println!("q = {:?}", q);
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
            let mut row = [F::zero(); NUM_WEIERSTRASS_ADD_COLS];
            let cols: &mut WeierstrassAddAssignCols<F> = unsafe { std::mem::transmute(&mut row) };
            let zero = BigUint::zero();
            Self::populate_fp_ops(cols, zero.clone(), zero.clone(), zero.clone(), zero);
            row
        });

        // Convert the trace to a row major matrix.
        RowMajorMatrix::new(
            rows.into_iter().flatten().collect::<Vec<_>>(),
            NUM_WEIERSTRASS_ADD_COLS,
        )
    }
}

impl<F, E: EllipticCurve, WP: WeierstrassParameters> BaseAir<F>
    for WeierstrassAddAssignChip<E, WP>
{
    fn width(&self) -> usize {
        NUM_WEIERSTRASS_ADD_COLS
    }
}

impl<AB, E: EllipticCurve, WP: WeierstrassParameters> Air<AB> for WeierstrassAddAssignChip<E, WP>
where
    AB: CurtaAirBuilder,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let row: &WeierstrassAddAssignCols<AB::Var> = main.row_slice(0).borrow();

        let p_x = limbs_from_prev_access(&row.p_access[0..8]);
        let p_y = limbs_from_prev_access(&row.p_access[8..16]);

        let q_x = limbs_from_prev_access(&row.q_access[0..8]);
        let q_y = limbs_from_prev_access(&row.q_access[8..16]);

        // *Printf
        //
        // This Secp eval fails even for padded rows, so i'm just double checking everything.
        //
        // This checks whether p, q, slope_numerators are all 0.
        //
        // This check passes (i.e., they are indeed all 0). And the slope_numerator.eval below fails
        // for padded rows.
        for i in 0..8 {
            builder
                .assert_zero(p_x.0[4 * i] + p_x.0[4 * i + 1] + p_x.0[4 * i + 2] + p_x.0[4 * i + 3]);
            builder
                .assert_zero(p_y.0[4 * i] + p_y.0[4 * i + 1] + p_y.0[4 * i + 2] + p_y.0[4 * i + 3]);
            builder
                .assert_zero(q_x.0[4 * i] + q_x.0[4 * i + 1] + q_x.0[4 * i + 2] + q_x.0[4 * i + 3]);
            builder
                .assert_zero(q_y.0[4 * i] + q_y.0[4 * i + 1] + q_y.0[4 * i + 2] + q_y.0[4 * i + 3]);
            builder.assert_zero(
                row.slope_numerator.result.0[4 * i]
                    + row.slope_numerator.result.0[4 * i + 1]
                    + row.slope_numerator.result.0[4 * i + 2]
                    + row.slope_numerator.result.0[4 * i + 3],
            );
        }

        // For whatever reason, this fails! The above check ensures that q_y = p_y = 0.
        row.slope_numerator
            .eval::<AB, E::BaseField, _, _>(builder, &q_y, &p_y, FpOperation::Sub);

        // // Slope = (q.y - p.y) / (q.x - p.x).
        // let slope = {
        //     row.slope_denominator.eval::<AB, E::BaseField, _, _>(
        //         builder,
        //         &q_x,
        //         &p_x,
        //         FpOperation::Sub,
        //     );

        //     row.slope.eval::<AB, E::BaseField, _, _>(
        //         builder,
        //         &row.slope_numerator.result,
        //         &row.slope_denominator.result,
        //         FpOperation::Div,
        //     );

        //     row.slope.result
        // };

        // // x = slope * slope - self.x - other.x
        // let x = {
        //     row.slope_squared.eval::<AB, E::BaseField, _, _>(
        //         builder,
        //         &slope,
        //         &slope,
        //         FpOperation::Mul,
        //     );

        //     row.p_x_plus_q_x
        //         .eval::<AB, E::BaseField, _, _>(builder, &p_x, &q_x, FpOperation::Add);

        //     row.x3_ins.eval::<AB, E::BaseField, _, _>(
        //         builder,
        //         &row.slope_squared.result,
        //         &row.p_x_plus_q_x.result,
        //         FpOperation::Sub,
        //     );

        //     row.x3_ins.result
        // };

        // // y = slope * (p.x - x_3n) - q.y
        // {
        //     row.p_x_minus_x
        //         .eval::<AB, E::BaseField, _, _>(builder, &p_x, &x, FpOperation::Sub);

        //     row.slope_times_p_x_minus_x.eval::<AB, E::BaseField, _, _>(
        //         builder,
        //         &slope,
        //         &row.p_x_minus_x.result,
        //         FpOperation::Mul,
        //     );

        //     row.y3_ins.eval::<AB, E::BaseField, _, _>(
        //         builder,
        //         &row.slope_times_p_x_minus_x.result,
        //         &p_y,
        //         FpOperation::Sub,
        //     );
        // }

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
    fn test_secp_add_simple() {
        setup_logger();
        let program = Program::from_elf("../programs/secp_add");
        prove(program);
    }
}
