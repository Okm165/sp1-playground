use rrs_lib::instruction_formats::{
    BType, IType, ITypeCSR, ITypeShamt, JType, RType, SType, UType,
};
use rrs_lib::InstructionProcessor;

use super::{Opcode, Register};

/// An instruction specifies an operation to execute and the operands.
#[derive(Debug, Clone, Copy)]
pub struct Instruction {
    pub opcode: Opcode,
    pub op_a: u32,
    pub op_b: u32,
    pub op_c: u32,
    pub imm_b: bool,
    pub imm_c: bool,
}

impl Instruction {
    /// Create a new instruction.
    pub fn new(opcode: Opcode, a: u32, b: u32, c: u32, imm_b: bool, imm_c: bool) -> Self {
        Self {
            opcode,
            op_a: a,
            op_b: b,
            op_c: c,
            imm_b,
            imm_c,
        }
    }

    /// Create a new instruction from an R-type instruction.
    fn from_r_type(opcode: Opcode, dec_insn: RType) -> Self {
        Self::new(
            opcode,
            dec_insn.rd as u32,
            dec_insn.rs1 as u32,
            dec_insn.rs2 as u32,
            false,
            false,
        )
    }

    /// Create a new instruction from an I-type instruction.
    fn from_i_type(opcode: Opcode, dec_insn: IType) -> Self {
        Self::new(
            opcode,
            dec_insn.rd as u32,
            dec_insn.rs1 as u32,
            dec_insn.imm as u32,
            false,
            true,
        )
    }

    /// Create a new instruction from an I-type instruction with a shamt.
    fn from_i_type_shamt(opcode: Opcode, dec_insn: ITypeShamt) -> Self {
        Self::new(
            opcode,
            dec_insn.rd as u32,
            dec_insn.rs1 as u32,
            dec_insn.shamt as u32,
            true,
            false,
        )
    }

    /// Create a new instruction from an S-type instruction.
    fn from_s_type(opcode: Opcode, dec_insn: SType) -> Self {
        Self::new(
            opcode,
            dec_insn.rs2 as u32,
            dec_insn.rs1 as u32,
            dec_insn.imm as u32,
            false,
            true,
        )
    }

    /// Create a new instruction from a B-type instruction.
    fn from_b_type(opcode: Opcode, dec_insn: BType) -> Self {
        Self::new(
            opcode,
            dec_insn.rs2 as u32,
            dec_insn.rs1 as u32,
            dec_insn.imm as u32,
            false,
            true,
        )
    }

    /// Create a new instruction from a J-type instruction.
    fn from_j_type(opcode: Opcode, dec_isn: JType) -> Self {
        Self::new(
            opcode,
            dec_isn.rd as u32,
            dec_isn.imm as u32,
            0,
            true,
            false,
        )
    }

    /// Create a new instruction that is not implemented.
    fn unimp() -> Self {
        Self::new(Opcode::UNIMP, 0, 0, 0, false, false)
    }

    /// Returns if the instruction is an ALU instruction.
    pub fn is_alu_instruction(&self) -> bool {
        match self.opcode {
            Opcode::ADD
            | Opcode::SUB
            | Opcode::XOR
            | Opcode::OR
            | Opcode::AND
            | Opcode::SLL
            | Opcode::SRL
            | Opcode::SRA
            | Opcode::SLT
            | Opcode::SLTU => true,
            _ => false,
        }
    }

    /// Returns if the instruction is a load instruction.
    pub fn is_load_instruction(&self) -> bool {
        match self.opcode {
            Opcode::LB | Opcode::LH | Opcode::LW | Opcode::LBU | Opcode::LHU => true,
            _ => false,
        }
    }

    /// Returns if the instruction is an R-type instruction.
    pub fn is_r_type(&self) -> bool {
        !self.imm_c
    }

    /// Returns whether the instruction is an I-type instruction.
    pub fn is_i_type(&self) -> bool {
        self.imm_c
    }

    /// Decode the instruction in the R-type format.
    pub fn r_type(&self) -> (Register, Register, Register) {
        (
            Register::from_u32(self.op_a),
            Register::from_u32(self.op_b),
            Register::from_u32(self.op_c),
        )
    }

    /// Decode the instruction in the I-type format.
    pub fn i_type(&self) -> (Register, Register, u32) {
        (
            Register::from_u32(self.op_a),
            Register::from_u32(self.op_b),
            self.op_c,
        )
    }

    /// Decode the instruction in the S-type format.
    pub fn s_type(&self) -> (Register, Register, u32) {
        (
            Register::from_u32(self.op_a),
            Register::from_u32(self.op_b),
            self.op_c,
        )
    }

    /// Decode the instruction in the B-type format.
    pub fn b_type(&self) -> (Register, Register, u32) {
        (
            Register::from_u32(self.op_a),
            Register::from_u32(self.op_b),
            self.op_c,
        )
    }

    /// Decode the instruction in the J-type format.
    pub fn j_type(&self) -> (Register, u32) {
        (Register::from_u32(self.op_a), self.op_b)
    }

    /// Decode the instruction in the U-type format.
    pub fn u_type(&self) -> (Register, u32) {
        (Register::from_u32(self.op_a), self.op_b)
    }
}

pub struct InstructionDecoder;

impl InstructionProcessor for InstructionDecoder {
    type InstructionResult = Instruction;

    fn process_add(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::ADD, dec_insn)
    }

    fn process_addi(&mut self, dec_insn: IType) -> Self::InstructionResult {
        Instruction::from_i_type(Opcode::ADD, dec_insn)
    }

    fn process_sub(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::SUB, dec_insn)
    }

    fn process_xor(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::XOR, dec_insn)
    }

    fn process_xori(&mut self, dec_insn: IType) -> Self::InstructionResult {
        Instruction::from_i_type(Opcode::XOR, dec_insn)
    }

    fn process_or(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::OR, dec_insn)
    }

    fn process_ori(&mut self, dec_insn: IType) -> Self::InstructionResult {
        Instruction::from_i_type(Opcode::OR, dec_insn)
    }

    fn process_and(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::AND, dec_insn)
    }

    fn process_andi(&mut self, dec_insn: IType) -> Self::InstructionResult {
        Instruction::from_i_type(Opcode::AND, dec_insn)
    }

    fn process_sll(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::SLL, dec_insn)
    }

    fn process_slli(&mut self, dec_insn: ITypeShamt) -> Self::InstructionResult {
        Instruction::from_i_type_shamt(Opcode::SLL, dec_insn)
    }

    fn process_srl(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::SRL, dec_insn)
    }

    fn process_srli(&mut self, dec_insn: ITypeShamt) -> Self::InstructionResult {
        Instruction::from_i_type_shamt(Opcode::SRL, dec_insn)
    }

    fn process_sra(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::SRA, dec_insn)
    }

    fn process_srai(&mut self, dec_insn: ITypeShamt) -> Self::InstructionResult {
        Instruction::from_i_type_shamt(Opcode::SRA, dec_insn)
    }

    fn process_slt(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::SLT, dec_insn)
    }

    fn process_slti(&mut self, dec_insn: IType) -> Self::InstructionResult {
        Instruction::from_i_type(Opcode::SLT, dec_insn)
    }

    fn process_sltu(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::SLTU, dec_insn)
    }

    fn process_sltui(&mut self, dec_insn: IType) -> Self::InstructionResult {
        Instruction::from_i_type(Opcode::SLTU, dec_insn)
    }

    fn process_lb(&mut self, dec_insn: IType) -> Self::InstructionResult {
        Instruction::from_i_type(Opcode::LB, dec_insn)
    }

    fn process_lh(&mut self, dec_insn: IType) -> Self::InstructionResult {
        Instruction::from_i_type(Opcode::LH, dec_insn)
    }

    fn process_lw(&mut self, dec_insn: IType) -> Self::InstructionResult {
        Instruction::from_i_type(Opcode::LW, dec_insn)
    }

    fn process_lbu(&mut self, dec_insn: IType) -> Self::InstructionResult {
        Instruction::from_i_type(Opcode::LBU, dec_insn)
    }

    fn process_lhu(&mut self, dec_insn: IType) -> Self::InstructionResult {
        Instruction::from_i_type(Opcode::LHU, dec_insn)
    }

    fn process_sb(&mut self, dec_insn: SType) -> Self::InstructionResult {
        Instruction::from_s_type(Opcode::SB, dec_insn)
    }

    fn process_sh(&mut self, dec_insn: SType) -> Self::InstructionResult {
        Instruction::from_s_type(Opcode::SH, dec_insn)
    }

    fn process_sw(&mut self, dec_insn: SType) -> Self::InstructionResult {
        Instruction::from_s_type(Opcode::SW, dec_insn)
    }

    fn process_beq(&mut self, dec_insn: BType) -> Self::InstructionResult {
        Instruction::from_b_type(Opcode::BEQ, dec_insn)
    }

    fn process_bne(&mut self, dec_insn: BType) -> Self::InstructionResult {
        Instruction::from_b_type(Opcode::BNE, dec_insn)
    }

    fn process_blt(&mut self, dec_insn: BType) -> Self::InstructionResult {
        Instruction::from_b_type(Opcode::BLT, dec_insn)
    }

    fn process_bge(&mut self, dec_insn: BType) -> Self::InstructionResult {
        Instruction::from_b_type(Opcode::BGE, dec_insn)
    }

    fn process_bltu(&mut self, dec_insn: BType) -> Self::InstructionResult {
        Instruction::from_b_type(Opcode::BLTU, dec_insn)
    }

    fn process_bgeu(&mut self, dec_insn: BType) -> Self::InstructionResult {
        Instruction::from_b_type(Opcode::BGEU, dec_insn)
    }

    fn process_jal(&mut self, dec_insn: JType) -> Self::InstructionResult {
        Instruction::from_j_type(Opcode::JAL, dec_insn)
    }

    fn process_jalr(&mut self, dec_insn: IType) -> Self::InstructionResult {
        Instruction::from_i_type(Opcode::JALR, dec_insn)
    }

    /// LUI instructions are converted to an SLL instruction with imm_b and imm_c turned on.
    /// Additionally the op_c should be set to 12.
    fn process_lui(&mut self, dec_insn: UType) -> Self::InstructionResult {
        Instruction::new(
            Opcode::SLL,
            dec_insn.rd as u32,
            dec_insn.imm as u32,
            12,
            true,
            true,
        )
    }

    /// AUIPC instructions have the third operand set to imm << 12.
    fn process_auipc(&mut self, dec_insn: UType) -> Self::InstructionResult {
        Instruction::new(
            Opcode::AUIPC,
            dec_insn.rd as u32,
            dec_insn.imm as u32,
            (dec_insn.imm << 12) as u32,
            true,
            true,
        )
    }

    fn process_ecall(&mut self) -> Self::InstructionResult {
        todo!()
    }

    fn process_ebreak(&mut self) -> Self::InstructionResult {
        todo!()
    }

    fn process_mul(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::MUL, dec_insn)
    }

    fn process_mulh(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::MULH, dec_insn)
    }

    fn process_mulhu(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::MULHU, dec_insn)
    }

    fn process_mulhsu(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::MULHSU, dec_insn)
    }

    fn process_div(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::DIV, dec_insn)
    }

    fn process_divu(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::DIVU, dec_insn)
    }

    fn process_rem(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::REM, dec_insn)
    }

    fn process_remu(&mut self, dec_insn: RType) -> Self::InstructionResult {
        Instruction::from_r_type(Opcode::REMU, dec_insn)
    }

    fn process_csrrc(&mut self, _: ITypeCSR) -> Self::InstructionResult {
        Instruction::unimp()
    }

    fn process_csrrci(&mut self, _: ITypeCSR) -> Self::InstructionResult {
        Instruction::unimp()
    }

    fn process_csrrs(&mut self, _: ITypeCSR) -> Self::InstructionResult {
        Instruction::unimp()
    }

    fn process_csrrsi(&mut self, _: ITypeCSR) -> Self::InstructionResult {
        Instruction::unimp()
    }

    fn process_csrrw(&mut self, _: ITypeCSR) -> Self::InstructionResult {
        Instruction::unimp()
    }

    fn process_csrrwi(&mut self, _: ITypeCSR) -> Self::InstructionResult {
        Instruction::unimp()
    }

    fn process_fence(&mut self, _: IType) -> Self::InstructionResult {
        Instruction::unimp()
    }

    fn process_mret(&mut self) -> Self::InstructionResult {
        Instruction::unimp()
    }

    fn process_wfi(&mut self) -> Self::InstructionResult {
        Instruction::unimp()
    }
}
