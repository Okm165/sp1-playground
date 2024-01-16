use crate::air::{BaseAirBuilder, CurtaAirBuilder, Word, WordAirBuilder};
use crate::bytes::ByteOpcode;

use core::borrow::{Borrow, BorrowMut};
use core::mem::{size_of, transmute};
use p3_air::Air;
use p3_air::AirBuilder;
use p3_air::BaseAir;
use p3_field::{AbstractField, PrimeField32};
use p3_matrix::MatrixRowSlices;
use p3_util::indices_arr;
use std::mem::transmute_copy;
use valida_derive::AlignedBorrow;

use super::instruction_cols::InstructionCols;
use super::opcode_cols::OpcodeSelectors;
use super::trace::CpuChip;
use crate::runtime::{AccessPosition, Opcode};

#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryReadCols<T> {
    pub value: Word<T>,
    pub segment: T,
    pub timestamp: T,
}

#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryAccessCols<T> {
    pub value: Word<T>,
    pub prev_value: Word<T>,
    // The previous segment and timestamp that this memory access is being read from.
    pub segment: T,
    pub timestamp: T,
}
#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryColumns<T> {
    // An addr that we are reading from or writing to as a word. We are guaranteed that this does
    // not overflow the field when reduced.
    pub addr_word: Word<T>,
    pub addr_aligned: T,
    pub addr_offset: T,
    pub memory_access: MemoryAccessCols<T>,
}

#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct BranchColumns<T> {
    pub pc: Word<T>,
    pub next_pc: Word<T>,

    pub a_eq_b: T,
    pub a_gt_b: T,
    pub a_lt_b: T,
}

#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct JumpColumns<T> {
    pub pc: Word<T>, // These are needed for JAL
    pub next_pc: Word<T>,
}

#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct AUIPCColumns<T> {
    pub pc: Word<T>,
}

/// An AIR table for memory accesses.
#[derive(AlignedBorrow, Default, Debug)]
#[repr(C)]
pub struct CpuCols<T> {
    /// The current segment.
    pub segment: T,
    /// The clock cycle value.
    pub clk: T,
    // /// The program counter value.
    pub pc: T,

    // Columns related to the instruction.
    pub instruction: InstructionCols<T>,
    // Selectors for the opcode.
    pub selectors: OpcodeSelectors<T>,

    // Operand values, either from registers or immediate values.
    pub op_a_access: MemoryAccessCols<T>,
    pub op_b_access: MemoryAccessCols<T>,
    pub op_c_access: MemoryAccessCols<T>,

    // This is transmuted to MemoryColumns or BNEColumns
    pub opcode_specific_columns: [T; OPCODE_SPECIFIC_COLUMNS_SIZE],

    // This column is set by the trace generator to indicate that the row's opcode is branch related
    // AND branch condition is true.
    // It is an opcode_specific_column, but it is needed for a multiplicity condition, which must be
    // a degree of 1, so can't be embedded within the opcode_specific_columns.
    pub branching: T,

    // Selector to label whether this row is a non padded row.
    pub is_real: T,
}

pub(crate) const NUM_CPU_COLS: usize = size_of::<CpuCols<u8>>();

pub(crate) const CPU_COL_MAP: CpuCols<usize> = make_col_map();
const fn make_col_map() -> CpuCols<usize> {
    let indices_arr = indices_arr::<NUM_CPU_COLS>();
    unsafe { transmute::<[usize; NUM_CPU_COLS], CpuCols<usize>>(indices_arr) }
}

pub(crate) const OPCODE_SPECIFIC_COLUMNS_SIZE: usize = get_opcode_specific_columns_offset();
// This is a constant function, so we can't have it dynamically return the largest opcode specific
// struct size.
const fn get_opcode_specific_columns_offset() -> usize {
    let memory_columns_size = size_of::<MemoryColumns<u8>>();
    let branch_columns_size = size_of::<BranchColumns<u8>>();
    let jump_columns_size = size_of::<JumpColumns<u8>>();
    let aui_pc_columns_size = size_of::<AUIPCColumns<u8>>();

    let return_val = memory_columns_size;

    if branch_columns_size > return_val {
        panic!("BranchColumns is too large to fit in the opcode_specific_columns array.");
    }

    if jump_columns_size > return_val {
        panic!("JumpColumns is too large to fit in the opcode_specific_columns array.");
    }

    if aui_pc_columns_size > return_val {
        panic!("AUIPCColumns is too large to fit in the opcode_specific_columns array.");
    }

    return_val
}

impl CpuCols<u32> {
    pub fn from_trace_row<F: PrimeField32>(row: &[F]) -> Self {
        let sized: [u32; NUM_CPU_COLS] = row
            .iter()
            .map(|x| x.as_canonical_u32())
            .collect::<Vec<u32>>()
            .try_into()
            .unwrap();
        unsafe { transmute::<[u32; NUM_CPU_COLS], CpuCols<u32>>(sized) }
    }
}

impl<T> CpuCols<T> {
    pub fn op_a_val(&self) -> &Word<T> {
        &self.op_a_access.value
    }

    pub fn op_b_val(&self) -> &Word<T> {
        &self.op_b_access.value
    }

    pub fn op_c_val(&self) -> &Word<T> {
        &self.op_c_access.value
    }
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

        // Dummy constraint of degree 3.
        builder.assert_eq(
            local.pc * local.pc * local.pc,
            local.pc * local.pc * local.pc,
        );

        builder.assert_bool(local.is_real);

        // Clock constraints
        builder.when_first_row().assert_one(local.clk);

        // TODO: handle precompile dynamic clk
        // builder
        //     .when_transition()
        //     .assert_eq(local.clk + AB::F::from_canonical_u32(4), next.clk);

        // Contrain the interaction with program table
        builder.send_program(local.pc, local.instruction, local.selectors, local.is_real);

        let is_memory_instruction: AB::Expr = self.is_memory_instruction::<AB>(&local.selectors);
        let is_branch_instruction: AB::Expr = self.is_branch_instruction::<AB>(&local.selectors);
        let is_alu_instruction: AB::Expr = self.is_alu_instruction::<AB>(&local.selectors);

        //////////////////////////////////////////

        // Constraint op_a_val, op_b_val, op_c_val
        // Constraint the op_b_val and op_c_val columns when imm_b and imm_c are true.
        builder
            .when(local.selectors.imm_b)
            .assert_word_eq(*local.op_b_val(), local.instruction.op_b);
        builder
            .when(local.selectors.imm_c)
            .assert_word_eq(*local.op_c_val(), local.instruction.op_c);

        // // We always write to the first register unless we are doing a branch_op or a store_op.
        // // The multiplicity is 1-selectors.noop-selectors.reg_0_write (the case where we're trying to write to register 0).
        builder.constraint_memory_access(
            local.segment,
            local.clk + AB::F::from_canonical_u32(AccessPosition::A as u32),
            local.instruction.op_a[0],
            local.op_a_access,
            AB::Expr::one() - local.selectors.is_noop - local.selectors.reg_0_write,
        );

        builder
            .when(is_branch_instruction.clone() + local.selectors.is_store)
            .assert_word_eq(*local.op_a_val(), local.op_a_access.prev_value);

        // // We always read to register b and register c unless the imm_b or imm_c flags are set.
        // TODO: for these, we could save the "op_b_access.prev_value" column because it's always
        // a read and never a write.
        builder.constraint_memory_access(
            local.segment,
            local.clk + AB::F::from_canonical_u32(AccessPosition::B as u32),
            local.instruction.op_b[0],
            local.op_b_access,
            AB::Expr::one() - local.selectors.imm_b,
        );
        builder
            .when(AB::Expr::one() - local.selectors.imm_b)
            .assert_word_eq(*local.op_b_val(), local.op_b_access.prev_value);

        builder.constraint_memory_access(
            local.segment,
            local.clk + AB::F::from_canonical_u32(AccessPosition::C as u32),
            local.instruction.op_c[0],
            local.op_c_access,
            AB::Expr::one() - local.selectors.imm_c,
        );
        builder
            .when(AB::Expr::one() - local.selectors.imm_c)
            .assert_word_eq(*local.op_c_val(), local.op_c_access.prev_value);

        let memory_columns: MemoryColumns<AB::Var> =
            unsafe { transmute_copy(&local.opcode_specific_columns) };

        builder.constraint_memory_access(
            local.segment,
            local.clk + AB::F::from_canonical_u32(AccessPosition::Memory as u32),
            memory_columns.addr_aligned,
            memory_columns.memory_access,
            is_memory_instruction.clone(),
        );

        //////////////////////////////////////////

        // Check that local.addr_offset \in [0, WORD_SIZE) by byte range checking local.addr_offset << 6
        // and local.addr_offset.
        builder.send_byte(
            AB::Expr::from_canonical_u8(ByteOpcode::Range as u8),
            AB::Expr::zero(),
            memory_columns.addr_offset,
            memory_columns.addr_offset * AB::F::from_canonical_u8(64),
            is_memory_instruction.clone(),
        );

        // Check that reduce(addr_word) == addr_aligned + addr_offset
        builder
            .when(is_memory_instruction.clone())
            .assert_eq::<AB::Expr, AB::Expr>(
                memory_columns.addr_aligned + memory_columns.addr_offset,
                memory_columns.addr_word.reduce::<AB>(),
            );

        // Check that each addr_word element is a byte
        builder.range_check_word(memory_columns.addr_word, is_memory_instruction.clone());

        // Send to the ALU table to verify correct calculation of addr_word
        builder.send_alu(
            AB::Expr::from_canonical_u32(Opcode::ADD as u32),
            memory_columns.addr_word,
            *local.op_b_val(),
            *local.op_c_val(),
            is_memory_instruction.clone(),
        );

        //////////////////////////////////////////

        //// Branch instructions
        self.branch_ops_eval::<AB>(builder, is_branch_instruction.clone(), local, next);

        //// Jump instructions
        self.jump_ops_eval::<AB>(builder, local, next);

        //// AUIPC instruction
        self.auipc_eval(builder, local);

        //// ALU instructions
        builder.send_alu(
            local.instruction.opcode,
            *local.op_a_val(),
            *local.op_b_val(),
            *local.op_c_val(),
            is_alu_instruction,
        );

        // TODO:  Need to handle HALT ecall
        // For all non branch or jump instructions, verify that next.pc == pc + 4
        // builder
        //     .when_not(is_branch_instruction + local.selectors.is_jal + local.selectors.is_jalr)
        //     .assert_eq(local.pc + AB::Expr::from_canonical_u8(4), next.pc);
    }
}

impl CpuChip {
    fn is_alu_instruction<AB: CurtaAirBuilder>(
        &self,
        opcode_selectors: &OpcodeSelectors<AB::Var>,
    ) -> AB::Expr {
        opcode_selectors.is_add
            + opcode_selectors.is_sub
            + opcode_selectors.is_mul
            + opcode_selectors.is_div
            + opcode_selectors.is_shift
            + opcode_selectors.is_bitwise
            + opcode_selectors.is_lt
    }

    fn is_memory_instruction<AB: CurtaAirBuilder>(
        &self,
        opcode_selectors: &OpcodeSelectors<AB::Var>,
    ) -> AB::Expr {
        opcode_selectors.is_load + opcode_selectors.is_store
    }

    fn is_branch_instruction<AB: CurtaAirBuilder>(
        &self,
        opcode_selectors: &OpcodeSelectors<AB::Var>,
    ) -> AB::Expr {
        opcode_selectors.is_beq
            + opcode_selectors.is_bne
            + opcode_selectors.is_blt
            + opcode_selectors.is_bge
            + opcode_selectors.is_bltu
            + opcode_selectors.is_bgeu
    }

    fn branch_ops_eval<AB: CurtaAirBuilder>(
        &self,
        builder: &mut AB,
        is_branch_instruction: AB::Expr,
        local: &CpuCols<AB::Var>,
        next: &CpuCols<AB::Var>,
    ) {
        //// This function will verify all the branching related columns.
        // It does this in few parts.
        // 1. It verifies that the next pc is correct based on the branching column.  That column
        //    is a boolean that indicates whether the branch condition is true.
        // 2. It verifies the correct value of branching based on the helper bool columns (a_eq_b,
        //    a_gt_b, a_lt_b).
        // 3. It verifier the correct values of the helper bool columns based on op_a and op_b.

        // Get the branch specific columns
        let branch_columns: BranchColumns<AB::Var> =
            unsafe { transmute_copy(&local.opcode_specific_columns) };

        //// Check that the new pc is calculated correctly.
        // First handle the case when local.branching == true

        // Verify that branch_columns.pc is correct.  That is local.pc in WORD form.
        // Note that when local.branching == True, then is_branch_instruction == True.
        builder
            .when(local.branching)
            .assert_eq(branch_columns.pc.reduce::<AB>(), local.pc);

        // Verify that branch_columns.next_pc is correct.  That is next.pc in WORD form.
        builder
            .when(local.branching)
            .assert_eq(branch_columns.next_pc.reduce::<AB>(), next.pc);

        // Calculate the new pc via the ADD chip if local.branching == true
        builder.send_alu(
            AB::Expr::from_canonical_u8(Opcode::ADD as u8),
            branch_columns.next_pc,
            branch_columns.pc,
            *local.op_c_val(),
            local.branching,
            // Note that if local.branching == 1 => is_branch_instruction == 1
            // We can't have an ADD clause of condition/selector columns here, since that would
            // require a multiply which would have a degree of > 1 (the max degree allowable for
            // 'multiplicity').
        );

        // Check that pc + 4 == next_pc if local.branching == false
        builder
            .when(is_branch_instruction.clone())
            .when_not(local.branching)
            .assert_eq(local.pc + AB::Expr::from_canonical_u8(4), next.pc);

        //// Check that the branching value is correct

        // Boolean range check local.branching
        builder
            .when(is_branch_instruction.clone())
            .assert_bool(local.branching);

        // Check that branching value is correct based on the opcode and the helper bools.
        builder
            .when(local.selectors.is_beq * local.branching)
            .assert_one(branch_columns.a_eq_b);
        builder
            .when(local.selectors.is_beq)
            .when_not(local.branching)
            .assert_one(branch_columns.a_gt_b + branch_columns.a_lt_b);

        builder
            .when(local.selectors.is_bne * local.branching)
            .assert_one(branch_columns.a_gt_b + branch_columns.a_lt_b);
        builder
            .when(local.selectors.is_bne)
            .when_not(local.branching)
            .assert_one(branch_columns.a_eq_b);

        builder
            .when((local.selectors.is_blt + local.selectors.is_bltu) * local.branching)
            .assert_one(branch_columns.a_lt_b);
        builder
            .when(local.selectors.is_blt + local.selectors.is_bltu)
            .when_not(local.branching)
            .assert_one(branch_columns.a_eq_b + branch_columns.a_gt_b);

        builder
            .when((local.selectors.is_bge + local.selectors.is_bgeu) * local.branching)
            .assert_one(branch_columns.a_gt_b);

        builder
            .when(local.selectors.is_bge + local.selectors.is_bgeu)
            .when_not(local.branching)
            .assert_one(branch_columns.a_eq_b + branch_columns.a_lt_b);

        //// Check that the helper bools' value is correct.
        builder
            .when(is_branch_instruction.clone() * branch_columns.a_eq_b)
            .assert_word_eq(*local.op_a_val(), *local.op_b_val());

        let use_signed_comparison = local.selectors.is_blt + local.selectors.is_bge;
        builder.send_alu(
            use_signed_comparison.clone() * AB::Expr::from_canonical_u8(Opcode::SLT as u8)
                + (AB::Expr::one() - use_signed_comparison.clone())
                    * AB::Expr::from_canonical_u8(Opcode::SLTU as u8),
            Word::extend::<AB>(branch_columns.a_lt_b),
            *local.op_a_val(),
            *local.op_b_val(),
            is_branch_instruction.clone(),
        );

        builder.send_alu(
            use_signed_comparison.clone() * AB::Expr::from_canonical_u8(Opcode::SLT as u8)
                + (AB::Expr::one() - use_signed_comparison)
                    * AB::Expr::from_canonical_u8(Opcode::SLTU as u8),
            Word::extend::<AB>(branch_columns.a_gt_b),
            *local.op_b_val(),
            *local.op_a_val(),
            is_branch_instruction.clone(),
        );
    }

    fn jump_ops_eval<AB: CurtaAirBuilder>(
        &self,
        builder: &mut AB,
        local: &CpuCols<AB::Var>,
        next: &CpuCols<AB::Var>,
    ) {
        // Get the jump specific columns
        let jump_columns: JumpColumns<AB::Var> =
            unsafe { transmute_copy(&local.opcode_specific_columns) };

        // Verify that the local.pc + 4 is saved in op_a for both jump instructions.
        builder
            .when(local.selectors.is_jal + local.selectors.is_jalr)
            .assert_eq(
                local.op_a_val().reduce::<AB>(),
                local.pc + AB::F::from_canonical_u8(4),
            );

        // Verify that the word form of local.pc is correct for JAL instructions.
        builder
            .when(local.selectors.is_jal)
            .assert_eq(jump_columns.pc.reduce::<AB>(), local.pc);

        // Verify that the word form of next.pc is correct for both jump instructions.
        builder
            .when(local.selectors.is_jal + local.selectors.is_jalr)
            .assert_eq(jump_columns.next_pc.reduce::<AB>(), next.pc);

        // Verify that the new pc is calculated correctly for JAL instructions.
        builder.send_alu(
            AB::Expr::from_canonical_u32(Opcode::ADD as u32),
            jump_columns.next_pc,
            jump_columns.pc,
            *local.op_b_val(),
            local.selectors.is_jal,
        );

        // Verify that the new pc is calculated correctly for JALR instructions.
        builder.send_alu(
            AB::Expr::from_canonical_u32(Opcode::ADD as u32),
            jump_columns.next_pc,
            *local.op_b_val(),
            *local.op_c_val(),
            local.selectors.is_jalr,
        );
    }

    fn auipc_eval<AB: CurtaAirBuilder>(&self, builder: &mut AB, local: &CpuCols<AB::Var>) {
        // Get the auipc specific columns
        let auipc_columns: AUIPCColumns<AB::Var> =
            unsafe { transmute_copy(&local.opcode_specific_columns) };

        // Verify that the word form of local.pc is correct.
        builder
            .when(local.selectors.is_auipc)
            .assert_eq(auipc_columns.pc.reduce::<AB>(), local.pc);

        // Verify that op_a == pc + op_b.
        builder.send_alu(
            AB::Expr::from_canonical_u32(Opcode::ADD as u32),
            *local.op_a_val(),
            auipc_columns.pc,
            *local.op_b_val(),
            local.selectors.is_auipc,
        );
    }
}
