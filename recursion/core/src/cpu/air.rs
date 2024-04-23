use crate::air::BinomialExtensionUtils;
use crate::air::BlockBuilder;
use crate::cpu::CpuChip;
use crate::runtime::RecursionProgram;
use core::mem::size_of;
use p3_air::Air;
use p3_air::AirBuilder;
use p3_air::BaseAir;
use p3_field::extension::BinomiallyExtendable;
use p3_field::AbstractField;
use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;
use sp1_core::air::AirInteraction;
use sp1_core::air::BinomialExtension;
use sp1_core::air::MachineAir;
use sp1_core::lookup::InteractionKind;
use sp1_core::utils::indices_arr;
use sp1_core::utils::pad_rows;
use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::mem::transmute;
use tracing::instrument;

use super::columns::CpuCols;
use crate::air::SP1RecursionAirBuilder;
use crate::runtime::ExecutionRecord;
use crate::runtime::D;

pub const NUM_CPU_COLS: usize = size_of::<CpuCols<u8>>();

const fn make_col_map() -> CpuCols<usize> {
    let indices_arr = indices_arr::<NUM_CPU_COLS>();
    unsafe { transmute::<[usize; NUM_CPU_COLS], CpuCols<usize>>(indices_arr) }
}

pub(crate) const CPU_COL_MAP: CpuCols<usize> = make_col_map();

impl<F: PrimeField32 + BinomiallyExtendable<D>> MachineAir<F> for CpuChip<F> {
    type Record = ExecutionRecord<F>;
    type Program = RecursionProgram<F>;

    fn name(&self) -> String {
        "CPU".to_string()
    }

    #[instrument(name = "generate cpu trace", level = "debug", skip_all)]
    fn generate_trace(
        &self,
        input: &ExecutionRecord<F>,
        _: &mut ExecutionRecord<F>,
    ) -> RowMajorMatrix<F> {
        let mut rows = input
            .cpu_events
            .iter()
            .map(|event| {
                let mut row = [F::zero(); NUM_CPU_COLS];
                let cols: &mut CpuCols<F> = row.as_mut_slice().borrow_mut();

                cols.clk = event.clk;
                cols.pc = event.pc;
                cols.fp = event.fp;

                cols.selectors.populate(&event.instruction);

                cols.instruction.opcode = F::from_canonical_u32(event.instruction.opcode as u32);
                cols.instruction.op_a = event.instruction.op_a;
                cols.instruction.op_b = event.instruction.op_b;
                cols.instruction.op_c = event.instruction.op_c;
                cols.instruction.imm_b = F::from_canonical_u32(event.instruction.imm_b as u32);
                cols.instruction.imm_c = F::from_canonical_u32(event.instruction.imm_c as u32);

                if let Some(record) = &event.a_record {
                    cols.a.populate(record);
                }
                if let Some(record) = &event.b_record {
                    cols.b.populate(record);
                } else {
                    cols.b.value = event.instruction.op_b;
                }
                if let Some(record) = &event.c_record {
                    cols.c.populate(record);
                } else {
                    cols.c.value = event.instruction.op_c;
                }

                let alu_cols = cols.opcode_specific.alu_mut();
                alu_cols.ext_a = cols.a.value;
                alu_cols.ext_b = cols.b.value;
                alu_cols.ext_c = cols.c.value;
                if cols.selectors.is_div.is_one() {
                    alu_cols.inverse_scratch = cols.c.value.0[0].inverse().into();
                } else if cols.selectors.is_ediv.is_one() {
                    alu_cols.inverse_scratch = BinomialExtension::from_block(cols.c.value)
                        .inverse()
                        .as_block();
                }

                // cols.a_eq_b
                //     .populate((cols.a.value.0[0] - cols.b.value.0[0]).as_canonical_u32());

                // let is_last_row = F::from_bool(i == input.cpu_events.len() - 1);
                // cols.beq = cols.is_beq * cols.a_eq_b.result * (F::one() - is_last_row);
                // cols.bne = cols.is_bne * (F::one() - cols.a_eq_b.result) * (F::one() - is_last_row);

                cols.is_real = F::one();
                row
            })
            .collect::<Vec<_>>();

        pad_rows(&mut rows, || {
            let mut row = [F::zero(); NUM_CPU_COLS];
            let cols: &mut CpuCols<F> = row.as_mut_slice().borrow_mut();
            cols.selectors.is_noop = F::one();
            row
        });

        let mut trace =
            RowMajorMatrix::new(rows.into_iter().flatten().collect::<Vec<_>>(), NUM_CPU_COLS);

        for i in input.cpu_events.len()..trace.height() {
            trace.values[i * NUM_CPU_COLS + CPU_COL_MAP.clk] =
                F::from_canonical_u32(4) * F::from_canonical_usize(i);
            trace.values[i * NUM_CPU_COLS + CPU_COL_MAP.instruction.imm_b] =
                F::from_canonical_u32(1);
            trace.values[i * NUM_CPU_COLS + CPU_COL_MAP.instruction.imm_c] =
                F::from_canonical_u32(1);
        }
        trace
    }

    fn included(&self, _: &Self::Record) -> bool {
        true
    }
}

impl<F: Send + Sync> BaseAir<F> for CpuChip<F> {
    fn width(&self) -> usize {
        NUM_CPU_COLS
    }
}

impl<AB> Air<AB> for CpuChip<AB::F>
where
    AB: SP1RecursionAirBuilder,
{
    fn eval(&self, builder: &mut AB) {
        // Constraints for the CPU chip.
        //
        // - Constraints for fetching the instruction.
        // - Constraints for incrementing the internal state consisting of the program counter
        //   and the clock.

        let main = builder.main();
        let (local, next) = (main.row_slice(0), main.row_slice(1));
        let local: &CpuCols<AB::Var> = (*local).borrow();
        let next: &CpuCols<AB::Var> = (*next).borrow();

        // Increment clk by 4 every cycle..
        builder
            .when_transition()
            .when(next.is_real)
            .assert_eq(local.clk.into() + AB::F::from_canonical_u32(4), next.clk);

        // // Increment pc by 1 every cycle unless it is a branch instruction that is satisfied.
        // builder
        //     .when_transition()
        //     .when(next.is_real * (AB::Expr::one() - (local.is_beq + local.is_bne)))
        //     .assert_eq(local.pc + AB::F::one(), next.pc);
        // builder
        //     .when(local.beq + local.bne)
        //     .assert_eq(next.pc, local.pc + local.c.value.0[0]);

        // Connect immediates.
        builder
            .when(local.instruction.imm_b)
            .assert_block_eq::<AB::Var, AB::Var>(local.b.value, local.instruction.op_b);
        builder
            .when(local.instruction.imm_c)
            .assert_block_eq::<AB::Var, AB::Var>(local.c.value, local.instruction.op_c);

        builder.assert_eq(
            local.is_real * local.is_real * local.is_real,
            local.is_real * local.is_real * local.is_real,
        );

        // // Compute extension ALU.
        // builder.assert_ext_eq(
        //     local.b.value.as_extension::<AB>() + local.c.value.as_extension::<AB>(),
        //     local.add_ext_scratch.as_extension::<AB>(),
        // );
        // builder.assert_ext_eq(
        //     local.b.value.as_extension::<AB>() - local.c.value.as_extension::<AB>(),
        //     local.sub_ext_scratch.as_extension::<AB>(),
        // );
        // builder.assert_ext_eq(
        //     local.b.value.as_extension::<AB>() * local.c.value.as_extension::<AB>(),
        //     local.mul_ext_scratch.as_extension::<AB>(),
        // );

        // // Connect ALU to CPU.
        // builder
        //     .when(local.is_add)
        //     .assert_eq(local.a.value.0[0], local.add_scratch);
        // builder
        //     .when(local.is_add)
        //     .assert_eq(local.a.value.0[1], AB::F::zero());
        // builder
        //     .when(local.is_add)
        //     .assert_eq(local.a.value.0[2], AB::F::zero());
        // builder
        //     .when(local.is_add)
        //     .assert_eq(local.a.value.0[3], AB::F::zero());

        // builder
        //     .when(local.is_sub)
        //     .assert_eq(local.a.value.0[0], local.sub_scratch);
        // builder
        //     .when(local.is_sub)
        //     .assert_eq(local.a.value.0[1], AB::F::zero());
        // builder
        //     .when(local.is_sub)
        //     .assert_eq(local.a.value.0[2], AB::F::zero());
        // builder
        //     .when(local.is_sub)
        //     .assert_eq(local.a.value.0[3], AB::F::zero());

        // builder
        //     .when(local.is_mul)
        //     .assert_eq(local.a.value.0[0], local.mul_scratch);
        // builder
        //     .when(local.is_mul)
        //     .assert_eq(local.a.value.0[1], AB::F::zero());
        // builder
        //     .when(local.is_mul)
        //     .assert_eq(local.a.value.0[2], AB::F::zero());
        // builder
        //     .when(local.is_mul)
        //     .assert_eq(local.a.value.0[3], AB::F::zero());

        // Compute if a == b.
        // IsZeroOperation::<AB::F>::eval::<AB>(
        //     builder,
        //     local.a.value.0[0] - local.b.value.0[0],
        //     local.a_eq_b,
        //     local.is_real.into(),
        // );

        // Receive C.
        builder.receive(AirInteraction::new(
            vec![
                local.c.addr.into(),
                local.c.prev_timestamp.into(),
                local.c.prev_value.0[0].into(),
                local.c.prev_value.0[1].into(),
                local.c.prev_value.0[2].into(),
                local.c.prev_value.0[3].into(),
            ],
            AB::Expr::one() - local.instruction.imm_c.into(),
            InteractionKind::Memory,
        ));
        builder.send(AirInteraction::new(
            vec![
                local.c.addr.into(),
                local.c.timestamp.into(),
                local.c.value.0[0].into(),
                local.c.value.0[1].into(),
                local.c.value.0[2].into(),
                local.c.value.0[3].into(),
            ],
            AB::Expr::one() - local.instruction.imm_c.into(),
            InteractionKind::Memory,
        ));

        // Receive B.
        builder.receive(AirInteraction::new(
            vec![
                local.b.addr.into(),
                local.b.prev_timestamp.into(),
                local.b.prev_value.0[0].into(),
                local.b.prev_value.0[1].into(),
                local.b.prev_value.0[2].into(),
                local.b.prev_value.0[3].into(),
            ],
            AB::Expr::one() - local.instruction.imm_b.into(),
            InteractionKind::Memory,
        ));
        builder.send(AirInteraction::new(
            vec![
                local.b.addr.into(),
                local.b.timestamp.into(),
                local.b.value.0[0].into(),
                local.b.value.0[1].into(),
                local.b.value.0[2].into(),
                local.b.value.0[3].into(),
            ],
            AB::Expr::one() - local.instruction.imm_b.into(),
            InteractionKind::Memory,
        ));

        // Receive A.
        builder.receive(AirInteraction::new(
            vec![
                local.a.addr.into(),
                local.a.prev_timestamp.into(),
                local.a.prev_value.0[0].into(),
                local.a.prev_value.0[1].into(),
                local.a.prev_value.0[2].into(),
                local.a.prev_value.0[3].into(),
            ],
            local.is_real.into(),
            InteractionKind::Memory,
        ));
        builder.send(AirInteraction::new(
            vec![
                local.a.addr.into(),
                local.a.timestamp.into(),
                local.a.value.0[0].into(),
                local.a.value.0[1].into(),
                local.a.value.0[2].into(),
                local.a.value.0[3].into(),
            ],
            local.is_real.into(),
            InteractionKind::Memory,
        ));

        // let mut prog_interaction_vals: Vec<AB::Expr> = vec![local.instruction.opcode.into()];
        // prog_interaction_vals.push(local.instruction.op_a.into());
        // prog_interaction_vals.extend_from_slice(&local.instruction.op_b.map(|x| x.into()).0);
        // prog_interaction_vals.extend_from_slice(&local.instruction.op_c.map(|x| x.into()).0);
        // prog_interaction_vals.push(local.instruction.imm_b.into());
        // prog_interaction_vals.push(local.instruction.imm_c.into());
        // prog_interaction_vals.extend_from_slice(
        //     &local
        //         .selectors
        //         .into_iter()
        //         .map(|x| x.into())
        //         .collect::<Vec<_>>(),
        // );
        // builder.send(AirInteraction::new(
        //     prog_interaction_vals,
        //     local.is_real.into(),
        //     InteractionKind::Program,
        // ));
    }
}
