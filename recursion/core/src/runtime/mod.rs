mod instruction;
mod opcode;
mod program;
mod record;

pub use instruction::*;
pub use opcode::*;
pub use program::*;
pub use record::*;

use crate::air::Word;
use crate::cpu::CpuEvent;
use crate::memory::MemoryRecord;

use p3_field::PrimeField32;
use sp1_core::runtime::AccessPosition;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct CpuRecord<F> {
    pub a: Option<MemoryRecord<F>>,
    pub b: Option<MemoryRecord<F>>,
    pub c: Option<MemoryRecord<F>>,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryEntry<F: PrimeField32> {
    pub value: Word<F>,
    pub timestamp: F,
}

pub struct Runtime<F: PrimeField32 + Clone> {
    /// The current clock.
    pub clk: F,

    /// The frame pointer.
    pub fp: F,

    /// The program counter.
    pub pc: F,

    /// The program.
    pub program: Program<F>,

    /// Memory.
    pub memory: Vec<MemoryEntry<F>>,

    /// The execution record.
    pub record: ExecutionRecord<F>,

    /// The access record for this cycle.
    pub access: CpuRecord<F>,
}

impl<F: PrimeField32 + Clone> Runtime<F> {
    pub fn new(program: &Program<F>) -> Self {
        let record = ExecutionRecord::<F> {
            program: Arc::new(program.clone()),
            ..Default::default()
        };
        Self {
            clk: F::zero(),
            program: program.clone(),
            fp: F::zero(),
            pc: F::zero(),
            memory: vec![MemoryEntry::default(); 1024 * 1024],
            record,
            access: CpuRecord::default(),
        }
    }

    fn mr(&mut self, addr: F, position: AccessPosition) -> F {
        let addr_usize = addr.as_canonical_u32() as usize;
        let timestamp = self.timestamp(&position);
        let entry = &self.memory[addr_usize];
        let (prev_value, prev_timestamp) = (entry.value, entry.timestamp);
        let record = MemoryRecord {
            addr,
            value: prev_value,
            timestamp,
            prev_value,
            prev_timestamp,
        };
        self.memory[addr_usize] = MemoryEntry {
            value: prev_value,
            timestamp,
        };
        match position {
            AccessPosition::A => self.access.a = Some(record),
            AccessPosition::B => self.access.b = Some(record),
            AccessPosition::C => self.access.c = Some(record),
            _ => unreachable!(),
        };
        prev_value.0[0]
    }

    fn mw(&mut self, addr: F, value: F, position: AccessPosition) {
        let addr_usize = addr.as_canonical_u32() as usize;
        let timestamp = self.timestamp(&position);
        let entry = &self.memory[addr_usize];
        let (prev_value, prev_timestamp) = (entry.value, entry.timestamp);
        let record = MemoryRecord {
            addr,
            value: Word::from(value),
            timestamp,
            prev_value,
            prev_timestamp,
        };
        self.memory[addr_usize] = MemoryEntry {
            value: Word::from(value),
            timestamp,
        };
        match position {
            AccessPosition::A => self.access.a = Some(record),
            AccessPosition::B => self.access.b = Some(record),
            AccessPosition::C => self.access.c = Some(record),
            _ => unreachable!(),
        };
    }

    fn timestamp(&self, position: &AccessPosition) -> F {
        self.clk + F::from_canonical_u32(*position as u32)
    }

    /// Fetch the destination address and input operand values for an ALU instruction.
    fn alu_rr(&mut self, instruction: &Instruction<F>) -> (F, F, F) {
        if !instruction.imm_c {
            let a_ptr = self.fp + instruction.op_a;
            let b_val = self.mr(self.fp + instruction.op_b, AccessPosition::B);
            let c_val = self.mr(self.fp + instruction.op_c, AccessPosition::C);
            (a_ptr, b_val, c_val)
        } else {
            let a_ptr = self.fp + instruction.op_a;
            let b_val = self.mr(self.fp + instruction.op_b, AccessPosition::B);
            let c_val = instruction.op_c;
            (a_ptr, b_val, c_val)
        }
    }

    /// Fetch the destination address input operand values for a load instruction (from heap).
    fn load_rr(&mut self, instruction: &Instruction<F>) -> (F, F) {
        if !instruction.imm_b {
            let a_ptr = self.fp + instruction.op_a;
            let b = self.mr(self.fp + instruction.op_b, AccessPosition::B);
            (a_ptr, b)
        } else {
            let a_ptr = self.fp + instruction.op_a;
            let b = instruction.op_b;
            (a_ptr, b)
        }
    }

    /// Fetch the destination address input operand values for a store instruction (from stack).
    fn store_rr(&mut self, instruction: &Instruction<F>) -> (F, F) {
        if !instruction.imm_b {
            let a_ptr = self.fp + instruction.op_a;
            let b = self.mr(self.fp + instruction.op_b, AccessPosition::B);
            (a_ptr, b)
        } else {
            let a_ptr = self.fp + instruction.op_a;
            (a_ptr, instruction.op_b)
        }
    }

    /// Fetch the input operand values for a branch instruction.
    fn branch_rr(&mut self, instruction: &Instruction<F>) -> (F, F, F) {
        let a = self.mr(self.fp + instruction.op_a, AccessPosition::A);
        let b = if !instruction.imm_b {
            self.mr(self.fp + instruction.op_b, AccessPosition::B)
        } else {
            instruction.op_b
        };
        let c = instruction.op_c;
        (a, b, c)
    }

    pub fn run(&mut self) {
        while self.pc < F::from_canonical_u32(self.program.instructions.len() as u32) {
            let idx = self.pc.as_canonical_u32() as usize;
            let instruction = self.program.instructions[idx].clone();
            let mut next_pc = self.pc + F::one();
            let (a, b, c): (F, F, F);
            match instruction.opcode {
                Opcode::ADD => {
                    let (a_ptr, b_val, c_val) = self.alu_rr(&instruction);
                    let a_val = b_val + c_val;
                    self.mw(a_ptr, a_val, AccessPosition::A);
                    (a, b, c) = (a_val, b_val, c_val);
                }
                Opcode::SUB => {
                    let (a_ptr, b_val, c_val) = self.alu_rr(&instruction);
                    let a_val = b_val - c_val;
                    self.mw(a_ptr, a_val, AccessPosition::A);
                    (a, b, c) = (a_val, b_val, c_val);
                }
                Opcode::MUL => {
                    let (a_ptr, b_val, c_val) = self.alu_rr(&instruction);
                    let a_val = b_val * c_val;
                    self.mw(a_ptr, a_val, AccessPosition::A);
                    (a, b, c) = (a_val, b_val, c_val);
                }
                Opcode::DIV => {
                    let (a_ptr, b_val, c_val) = self.alu_rr(&instruction);
                    let a_val = b_val / c_val;
                    self.mw(a_ptr, a_val, AccessPosition::A);
                    (a, b, c) = (a_val, b_val, c_val);
                }
                Opcode::LW => {
                    let (a_ptr, b_val) = self.load_rr(&instruction);
                    let a_val = b_val;
                    self.mw(a_ptr, a_val, AccessPosition::A);
                    (a, b, c) = (a_val, b_val, F::zero());
                }
                Opcode::SW => {
                    let (a_ptr, b_val) = self.store_rr(&instruction);
                    let a_val = b_val;
                    self.mw(a_ptr, a_val, AccessPosition::A);
                    (a, b, c) = (a_val, b_val, F::zero());
                }
                Opcode::BEQ => {
                    (a, b, c) = self.branch_rr(&instruction);
                    if a == b {
                        next_pc = c;
                    }
                }
                Opcode::BNE => {
                    (a, b, c) = self.branch_rr(&instruction);
                    if a != b {
                        next_pc = c;
                    }
                }
                Opcode::JAL => {
                    let imm = instruction.op_b;
                    let a_ptr = instruction.op_a + self.fp;
                    self.mw(a_ptr, self.pc, AccessPosition::A);
                    next_pc = self.pc + imm;
                    (a, b, c) = (a_ptr, F::zero(), F::zero());
                }
                Opcode::JALR => {
                    let imm = instruction.op_c;
                    let b_ptr = instruction.op_b + self.fp;
                    let a_ptr = instruction.op_a + self.fp;
                    let b_val = self.mr(b_ptr, AccessPosition::B);
                    let c_val = imm;
                    let a_val = self.pc + F::one();
                    self.mw(a_ptr, a_val, AccessPosition::A);
                    next_pc = b_val + c_val;
                    (a, b, c) = (a_val, b_val, c_val);
                }
            };

            let event = CpuEvent {
                clk: self.clk,
                pc: self.pc,
                fp: self.fp,
                instruction: instruction.clone(),
                a,
                a_record: self.access.a.clone(),
                b,
                b_record: self.access.b.clone(),
                c,
                c_record: self.access.c.clone(),
            };
            self.pc = next_pc;
            self.record.cpu_events.push(event);
            self.clk += F::from_canonical_u32(4);
        }

        // Collect all used memory addresses.
        for addr in 0..self.memory.len() {
            let entry = &self.memory[addr];
            if entry.timestamp != F::zero() {
                self.record
                    .first_memory_record
                    .push(F::from_canonical_usize(addr));

                self.record.last_memory_record.push((
                    F::from_canonical_usize(addr),
                    entry.timestamp,
                    entry.value,
                ))
            }
        }
    }
}
