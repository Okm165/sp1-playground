use crate::air::{CurtaAirBuilder, Word};

use core::borrow::{Borrow, BorrowMut};
use core::mem::{size_of, transmute};
use p3_air::Air;
use p3_air::AirBuilder;
use p3_air::BaseAir;
use p3_field::AbstractField;
use p3_matrix::MatrixRowSlices;
use p3_util::indices_arr;
use valida_derive::AlignedBorrow;

use super::instruction_cols::InstructionCols;
use super::opcode_cols::OpcodeSelectors;
use super::trace::CpuChip;

/// An AIR table for memory accesses.
#[derive(AlignedBorrow, Default)]
#[repr(C)]
pub struct CpuCols<T> {
    /// The clock cycle value.
    pub clk: T,
    // /// The program counter value.
    pub pc: T,

    // Columns related to the instruction.
    pub instruction: InstructionCols<T>,
    // Selectors for the opcode.
    pub selectors: OpcodeSelectors<T>,

    // // Operand values, either from registers or immediate values.
    pub op_a_val: Word<T>,
    pub op_b_val: Word<T>,
    pub op_c_val: Word<T>,

    // An addr that we are reading from or writing to.
    pub addr: Word<T>,
    // The associated memory value for `addr`.
    pub mem_val: Word<T>,

    // NOTE: This is actually a Bool<T>, but it might be easier to bus as a word for consistency with the register bus.
    pub branch_cond_val: Word<T>,
}

pub(crate) const NUM_CPU_COLS: usize = size_of::<CpuCols<u8>>();
pub(crate) const CPU_COL_MAP: CpuCols<usize> = make_col_map();

const fn make_col_map() -> CpuCols<usize> {
    let indices_arr = indices_arr::<NUM_CPU_COLS>();
    unsafe { transmute::<[usize; NUM_CPU_COLS], CpuCols<usize>>(indices_arr) }
}

impl<F> BaseAir<F> for CpuChip {
    fn width(&self) -> usize {
        NUM_CPU_COLS
    }
}

impl<AB> Air<AB> for CpuChip
where
    AB: CurtaAirBuilder,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local: &CpuCols<AB::Var> = main.row_slice(0).borrow();
        let next: &CpuCols<AB::Var> = main.row_slice(1).borrow();

        // Clock constraints
        builder.when_first_row().assert_one(local.clk);
        builder
            .when_transition()
            .assert_eq(local.clk + AB::Expr::one(), next.clk);

        // TODO: lookup (pc, opcode, op_a, op_b, op_c, ... all selectors) in the program table with multiplicity 1

        //// Constraint op_a_val, op_b_val, op_c_val
        // Constraint the op_b_val and op_c_val columns when imm_b and imm_c are true.
        for i in 0..4 {
            builder
                .when(local.selectors.imm_b)
                .assert_eq(local.op_b_val[i], local.instruction.op_b[i]);
            builder
                .when(local.selectors.imm_c)
                .assert_eq(local.op_c_val[i], local.instruction.op_c[i]);
        }

        builder.assert_eq(
            local.pc * local.pc * local.pc,
            local.pc * local.pc * local.pc,
        );

        //// For r-type, i-type and multiply instructions, we must constraint by an "opcode-oracle" table
        // TODO: lookup (clk, op_a_val, op_b_val, op_c_val) in the "opcode-oracle" table with multiplicity (register_instruction + immediate_instruction + multiply_instruction)

        //// For branch instructions
        // TODO: lookup (clk, branch_cond_val, op_a_val, op_b_val) in the "branch" table with multiplicity branch_instruction
        // Increment the pc by 4 + op_c_val * branch_cond_val where we interpret the first result as a bool that it is.

        // builder.when(local.selectors.branch_op).assert_eq(
        //     local.pc
        //         + AB::F::from_canonical_u8(4)
        //         + reduce::<AB>(local.op_c_val) * local.branch_cond_val.0[0],
        //     next.pc,
        // );

        // //// For jump instructions
        // builder
        //     .when(local.selectors.jalr + local.selectors.jal)
        //     .assert_eq(
        //         reduce::<AB>(local.op_a_val),
        //         local.pc + AB::F::from_canonical_u8(4),
        //     );
        // builder.when(local.selectors.jal).assert_eq(
        //     local.pc + AB::F::from_canonical_u8(4) + reduce::<AB>(local.op_b_val),
        //     next.pc,
        // );
        // builder.when(local.selectors.jalr).assert_eq(
        //     reduce::<AB>(local.op_b_val) + local.instruction.op_c,
        //     next.pc,
        // );

        // //// Upper immediate instructions
        // // lookup(clk, op_c_val, imm, 12) in SLT table with multiplicity AUIPC
        // builder.when(local.selectors.auipc).assert_eq(
        //     reduce::<AB>(local.op_a_val),
        //     reduce::<AB>(local.op_c_val) + local.pc,
        // );

        // Send interactions for all the ALUs.
        let ops = vec![
            local.selectors.add_op,
            local.selectors.sub_op,
            local.selectors.bitwise_op,
            local.selectors.shift_op,
            local.selectors.lt_op,
        ];
        for op in ops {
            builder.send_alu(
                local.instruction.opcode,
                local.op_a_val,
                local.op_b_val,
                local.op_c_val,
                op,
            );
        }
    }
}
