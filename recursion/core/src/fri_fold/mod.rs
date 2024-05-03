#![allow(clippy::needless_range_loop)]

use crate::air::RecursionMemoryAirBuilder;
use crate::memory::{MemoryReadCols, MemoryReadSingleCols, MemoryReadWriteCols};
use crate::runtime::Opcode;
use core::borrow::Borrow;
use itertools::Itertools;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::AbstractField;
use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;
use sp1_core::air::{BaseAirBuilder, BinomialExtension, ExtensionAirBuilder, MachineAir};
use sp1_core::utils::pad_rows_fixed;
use sp1_derive::AlignedBorrow;
use std::borrow::BorrowMut;
use tracing::instrument;

use crate::air::SP1RecursionAirBuilder;
use crate::memory::MemoryRecord;
use crate::runtime::{ExecutionRecord, RecursionProgram};

pub const NUM_FRI_FOLD_COLS: usize = core::mem::size_of::<FriFoldCols<u8>>();

#[derive(Default)]
pub struct FriFoldChip {
    pub fixed_log2_rows: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct FriFoldEvent<F> {
    pub is_last_iteration: F,
    pub clk: F,
    pub m: F,
    pub input_ptr: F,

    pub z: MemoryRecord<F>,
    pub alpha: MemoryRecord<F>,
    pub x: MemoryRecord<F>,
    pub log_height: MemoryRecord<F>,
    pub mat_opening_ptr: MemoryRecord<F>,
    pub ps_at_z_ptr: MemoryRecord<F>,
    pub alpha_pow_ptr: MemoryRecord<F>,
    pub ro_ptr: MemoryRecord<F>,

    pub p_at_x: MemoryRecord<F>,
    pub p_at_z: MemoryRecord<F>,

    pub alpha_pow_at_log_height: MemoryRecord<F>,
    pub ro_at_log_height: MemoryRecord<F>,
}

#[derive(AlignedBorrow, Debug, Clone, Copy)]
#[repr(C)]
pub struct FriFoldCols<T: Copy> {
    pub is_last_iteration: T,
    pub clk: T,

    /// The parameters into the FRI fold precompile.  These values are only read from memory.
    pub m: T,
    pub input_ptr: T,

    /// The inputs stored in memory.  All the values are just read from memory.
    pub z: MemoryReadCols<T>,
    pub alpha: MemoryReadCols<T>,
    pub x: MemoryReadSingleCols<T>,

    pub log_height: MemoryReadSingleCols<T>,
    pub mat_opening_ptr: MemoryReadSingleCols<T>,
    pub ps_at_z_ptr: MemoryReadSingleCols<T>,
    pub alpha_pow_ptr: MemoryReadSingleCols<T>,
    pub ro_ptr: MemoryReadSingleCols<T>,

    pub p_at_x: MemoryReadCols<T>,
    pub p_at_z: MemoryReadCols<T>,

    /// The values here are read and then written.
    pub alpha_pow_at_log_height: MemoryReadWriteCols<T>,
    pub ro_at_log_height: MemoryReadWriteCols<T>,

    pub is_real: T,
}

impl<F> BaseAir<F> for FriFoldChip {
    fn width(&self) -> usize {
        NUM_FRI_FOLD_COLS
    }
}

impl<F: PrimeField32> MachineAir<F> for FriFoldChip {
    type Record = ExecutionRecord<F>;

    type Program = RecursionProgram<F>;

    fn name(&self) -> String {
        "FriFold".to_string()
    }

    fn generate_dependencies(&self, _: &Self::Record, _: &mut Self::Record) {
        // This is a no-op.
    }

    #[instrument(name = "generate fri fold trace", level = "debug", skip_all, fields(rows = input.fri_fold_events.len()))]
    fn generate_trace(
        &self,
        input: &ExecutionRecord<F>,
        _: &mut ExecutionRecord<F>,
    ) -> RowMajorMatrix<F> {
        let mut rows = input
            .fri_fold_events
            .iter()
            .map(|event| {
                let mut row = [F::zero(); NUM_FRI_FOLD_COLS];

                let cols: &mut FriFoldCols<F> = row.as_mut_slice().borrow_mut();

                cols.is_last_iteration = event.is_last_iteration;
                cols.clk = event.clk;
                cols.m = event.m;
                cols.input_ptr = event.input_ptr;
                cols.is_real = F::one();

                cols.z.populate(&event.z);
                cols.alpha.populate(&event.alpha);
                cols.x.populate(&event.x);
                cols.log_height.populate(&event.log_height);
                cols.mat_opening_ptr.populate(&event.mat_opening_ptr);
                cols.ps_at_z_ptr.populate(&event.ps_at_z_ptr);
                cols.alpha_pow_ptr.populate(&event.alpha_pow_ptr);
                cols.ro_ptr.populate(&event.ro_ptr);

                cols.p_at_x.populate(&event.p_at_x);
                cols.p_at_z.populate(&event.p_at_z);

                cols.alpha_pow_at_log_height
                    .populate(&event.alpha_pow_at_log_height);
                cols.ro_at_log_height.populate(&event.ro_at_log_height);

                row
            })
            .collect_vec();

        // Pad the trace to a power of two.
        pad_rows_fixed(
            &mut rows,
            || [F::zero(); NUM_FRI_FOLD_COLS],
            self.fixed_log2_rows,
        );

        // Convert the trace to a row major matrix.
        let trace = RowMajorMatrix::new(rows.into_iter().flatten().collect(), NUM_FRI_FOLD_COLS);

        #[cfg(debug_assertions)]
        println!(
            "fri fold trace dims is width: {:?}, height: {:?}",
            trace.width(),
            trace.height()
        );

        trace
    }

    fn included(&self, record: &Self::Record) -> bool {
        !record.fri_fold_events.is_empty()
    }
}

impl FriFoldChip {
    pub fn eval_fri_fold<AB: BaseAirBuilder + ExtensionAirBuilder + RecursionMemoryAirBuilder>(
        &self,
        builder: &mut AB,
        local: &FriFoldCols<AB::Var>,
        next: &FriFoldCols<AB::Var>,
    ) {
        // Constraint that the operands are sent from the CPU table.
        builder.assert_bool(local.is_last_iteration);
        let operands = [
            local.clk.into() - local.m.into(),
            local.m.into() + AB::Expr::one(),
            local.input_ptr.into(),
            AB::Expr::zero(),
        ];
        builder.receive_table(
            Opcode::FRIFold.as_field::<AB::F>(),
            &operands,
            local.is_last_iteration,
        );

        builder
            .when_transition()
            .when(local.is_last_iteration)
            .when(next.is_real)
            .assert_zero(next.m);

        builder
            .when_transition()
            .when_not(local.is_last_iteration)
            .when(next.is_real)
            .assert_eq(next.m, local.m + AB::Expr::one());

        builder
            .when_transition()
            .when_not(local.is_last_iteration)
            .when(next.is_real)
            .assert_eq(local.input_ptr, next.input_ptr);

        builder
            .when_transition()
            .when_not(local.is_last_iteration)
            .when(next.is_real)
            .assert_eq(local.clk + AB::Expr::one(), next.clk);

        // Constrain read for `z` at `input_ptr`
        builder.recursion_eval_memory_access(
            local.clk,
            local.input_ptr + AB::Expr::zero(),
            &local.z,
            local.is_real,
        );

        // Constrain read for `alpha`
        builder.recursion_eval_memory_access(
            local.clk,
            local.input_ptr + AB::Expr::one(),
            &local.alpha,
            local.is_real,
        );

        // Constrain read for `x`
        builder.recursion_eval_memory_access_single(
            local.clk,
            local.input_ptr + AB::Expr::from_canonical_u32(2),
            &local.x,
            local.is_real,
        );

        // Constrain read for `log_height`
        builder.recursion_eval_memory_access_single(
            local.clk,
            local.input_ptr + AB::Expr::from_canonical_u32(3),
            &local.log_height,
            local.is_real,
        );

        // Constrain read for `mat_opening_ptr`
        builder.recursion_eval_memory_access_single(
            local.clk,
            local.input_ptr + AB::Expr::from_canonical_u32(4),
            &local.mat_opening_ptr,
            local.is_real,
        );

        // Constrain read for `ps_at_z_ptr`
        builder.recursion_eval_memory_access_single(
            local.clk,
            local.input_ptr + AB::Expr::from_canonical_u32(6),
            &local.ps_at_z_ptr,
            local.is_real,
        );

        // Constrain read for `alpha_pow_ptr`
        builder.recursion_eval_memory_access_single(
            local.clk,
            local.input_ptr + AB::Expr::from_canonical_u32(8),
            &local.alpha_pow_ptr,
            local.is_real,
        );

        // Constrain read for `ro_ptr`
        builder.recursion_eval_memory_access_single(
            local.clk,
            local.input_ptr + AB::Expr::from_canonical_u32(10),
            &local.ro_ptr,
            local.is_real,
        );

        // Constrain read for `p_at_x`
        builder.recursion_eval_memory_access(
            local.clk,
            local.mat_opening_ptr.access.value.into() + local.m.into(),
            &local.p_at_x,
            local.is_real,
        );

        // Constrain read for `p_at_z`
        builder.recursion_eval_memory_access(
            local.clk,
            local.ps_at_z_ptr.access.value.into() + local.m.into(),
            &local.p_at_z,
            local.is_real,
        );

        // Update alpha_pow_at_log_height.
        // 1. Constrain old and new value against memory
        builder.recursion_eval_memory_access(
            local.clk,
            local.alpha_pow_ptr.access.value.into() + local.log_height.access.value.into(),
            &local.alpha_pow_at_log_height,
            local.is_real,
        );

        // 2. Constrain new_value = old_value * alpha.
        let alpha = local.alpha.access.value.as_extension::<AB>();
        let alpha_pow_at_log_height = local
            .alpha_pow_at_log_height
            .prev_value
            .as_extension::<AB>();
        let new_alpha_pow_at_log_height = local
            .alpha_pow_at_log_height
            .access
            .value
            .as_extension::<AB>();

        builder.assert_ext_eq(
            alpha_pow_at_log_height.clone() * alpha,
            new_alpha_pow_at_log_height,
        );

        // Update ro_at_log_height.
        // 1. Constrain old and new value against memory.
        builder.recursion_eval_memory_access(
            local.clk,
            local.ro_ptr.access.value.into() + local.log_height.access.value.into(),
            &local.ro_at_log_height,
            local.is_real,
        );

        // 2. Constrain new_value = old_alpha_pow_at_log_height * quotient + old_value,
        // where quotient = (p_at_x - p_at_z) / (x - z)
        // <=> (new_value - old_value) * (z - x) = old_alpha_pow_at_log_height * (p_at_x - p_at_z)
        let p_at_z = local.p_at_z.access.value.as_extension::<AB>();
        let p_at_x = local.p_at_x.access.value.as_extension::<AB>();
        let z = local.z.access.value.as_extension::<AB>();
        let x = local.x.access.value.into();

        let ro_at_log_height = local.ro_at_log_height.prev_value.as_extension::<AB>();
        let new_ro_at_log_height = local.ro_at_log_height.access.value.as_extension::<AB>();
        builder.assert_ext_eq(
            (new_ro_at_log_height - ro_at_log_height) * (BinomialExtension::from_base(x) - z),
            (p_at_x - p_at_z) * alpha_pow_at_log_height,
        );
    }
}

impl<AB> Air<AB> for FriFoldChip
where
    AB: SP1RecursionAirBuilder,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let (local, next) = (main.row_slice(0), main.row_slice(1));
        let local: &FriFoldCols<AB::Var> = (*local).borrow();
        let next: &FriFoldCols<AB::Var> = (*next).borrow();
        self.eval_fri_fold::<AB>(builder, local, next);
    }
}
