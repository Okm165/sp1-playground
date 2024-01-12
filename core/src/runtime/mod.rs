mod instruction;
mod opcode;
mod program;
mod register;
mod segment;
mod syscall;

use crate::cpu::MemoryRecord;
use crate::precompiles::sha256::{ShaCompressEvent, ShaExtendEvent, SHA_COMPRESS_K};
use crate::{alu::AluEvent, cpu::CpuEvent};
pub use instruction::*;
pub use opcode::*;
pub use program::*;
pub use register::*;
pub use segment::*;
use std::collections::BTreeMap;
pub use syscall::*;

use p3_baby_bear::BabyBear;
use p3_field::AbstractField;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AccessPosition {
    Memory = 0,
    C = 1,
    B = 2,
    A = 3,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Record {
    a: Option<MemoryRecord>,
    b: Option<MemoryRecord>,
    c: Option<MemoryRecord>,
    memory: Option<MemoryRecord>,
}

/// An implementation of a runtime for the Curta VM.
///
/// The runtime is responsible for executing a user program and tracing important events which occur
/// during execution (i.e., memory reads, alu operations, etc).
///
/// For more information on the RV32IM instruction set, see the following:
/// https://www.cs.sfu.ca/~ashriram/Courses/CS295/assets/notebooks/RISCV/RISCV_CARD.pdf
#[allow(non_snake_case)]
pub struct Runtime {
    /// The global clock keeps track of how many instrutions have been executed through all segments.
    pub global_clk: u32,

    /// The clock keeps track of how many instructions have been executed in this segment.
    pub clk: u32,

    /// The program counter.
    pub pc: u32,

    /// The program.
    pub program: Program,

    /// The memory which instructions operate over.
    pub memory: BTreeMap<u32, u32>,

    /// Maps a memory address to (segment, timestamp) that it was touched.
    pub memory_access: BTreeMap<u32, (u32, u32)>,

    /// A stream of witnessed values (global to the entire program).
    pub witness: Vec<u32>,

    /// Segments
    pub segments: Vec<Segment>,

    /// The current segment for this section of the program.
    pub segment: Segment,

    /// The current record for the CPU event,
    pub record: Record,

    /// Global information needed for "global" chips, like the memory argument. It's a bit
    /// semantically incorrect to have this as a "Segment", since it's not really a segment
    /// in the traditional sense.
    pub global_segment: Segment,

    /// The maximum size of each segment.
    pub segment_size: u32,
}

impl Runtime {
    // Create a new runtime
    pub fn new(program: Program) -> Self {
        let mut segment = Segment::default();
        segment.program = program.clone();
        segment.index = 1;

        Self {
            global_clk: 0,
            clk: 0,
            pc: program.pc_start,
            program,
            memory: BTreeMap::new(),
            memory_access: BTreeMap::new(),
            witness: Vec::new(),
            segments: Vec::new(),
            segment,
            record: Record::default(),
            segment_size: 10000,
            global_segment: Segment::default(),
        }
    }

    /// Write to the witness stream.
    pub fn write_witness(&mut self, witness: &[u32]) {
        self.witness.extend(witness);
    }

    /// Get the current values of the registers.
    pub fn registers(&self) -> [u32; 32] {
        let mut registers = [0; 32];
        for i in 0..32 {
            let addr = Register::from_u32(i as u32) as u32;
            registers[i] = match self.memory.get(&addr) {
                Some(value) => *value,
                None => 0,
            };
        }
        return registers;
    }

    /// Get the current value of a register.
    pub fn register(&self, register: Register) -> u32 {
        let addr = register as u32;
        match self.memory.get(&addr) {
            Some(value) => *value,
            None => 0,
        }
    }

    /// Get the current value of a word.
    pub fn word(&self, addr: u32) -> u32 {
        match self.memory.get(&addr) {
            Some(value) => *value,
            None => 0,
        }
    }

    fn clk_from_position(&self, position: &AccessPosition) -> u32 {
        self.clk + *position as u32
    }

    fn current_segment(&self) -> u32 {
        self.segment.index
    }

    fn align(&self, addr: u32) -> u32 {
        addr - addr % 4
    }

    fn validate_memory_access(&self, addr: u32, position: AccessPosition) {
        if position == AccessPosition::Memory {
            assert_eq!(addr % 4, 0, "addr is not aligned");
            let _ = BabyBear::from_canonical_u32(addr);
            assert!(addr > 40); // Assert that the address is > the max register.
        } else {
            let _ = Register::from_u32(addr);
        }
    }

    /// Read from memory, assuming that all addresses are aligned.
    fn mr(&mut self, addr: u32, position: AccessPosition) -> u32 {
        self.validate_memory_access(addr, position);

        let value = self.memory.entry(addr).or_insert(0).clone();
        let (prev_segment, prev_timestamp) =
            self.memory_access.get(&addr).cloned().unwrap_or((0, 0));

        self.memory_access.insert(
            addr,
            (self.current_segment(), self.clk_from_position(&position)),
        );

        let record = MemoryRecord {
            value: value,
            segment: prev_segment,
            timestamp: prev_timestamp,
        };

        match position {
            AccessPosition::A => self.record.a = Some(record),
            AccessPosition::B => self.record.b = Some(record),
            AccessPosition::C => self.record.c = Some(record),
            AccessPosition::Memory => self.record.memory = Some(record),
        }
        value
    }

    /// Write to memory.
    /// We assume that we have called `mr` before on this addr before writing to memory for record keeping purposes.
    fn mw(&mut self, addr: u32, value: u32, position: AccessPosition) {
        self.validate_memory_access(addr, position);
        // Just update the value, since we assume that in the `mr` function we have updated the memory_access map appropriately.
        self.memory.insert(addr, value);

        assert!(self.memory_access.contains_key(&addr));
        // Make sure that we have updated the memory records appropriately.
        match position {
            AccessPosition::A => assert!(self.record.a.is_some()),
            AccessPosition::B => assert!(self.record.b.is_some()),
            AccessPosition::C => assert!(self.record.c.is_some()),
            AccessPosition::Memory => assert!(self.record.memory.is_some()),
        }
    }

    /// Read from register.
    fn rr(&mut self, register: Register, position: AccessPosition) -> u32 {
        self.mr(register as u32, position)
    }

    /// Write to register.
    fn rw(&mut self, register: Register, value: u32) {
        if register == Register::X0 {
            // We don't write to %x0. See 2.6 Load and Store Instruction on
            // P.18 of the RISC-V spec.
            return;
        }
        // Only for register writes, do we not read it before, so we put in the read here.
        self.mr(register as u32, AccessPosition::A);
        // The only time we are writing to a register is when it is register A.
        self.mw(register as u32, value, AccessPosition::A)
    }

    /// Emit a CPU event.
    fn emit_cpu(
        &mut self,
        segment: u32,
        clk: u32,
        pc: u32,
        instruction: Instruction,
        a: u32,
        b: u32,
        c: u32,
        memory_store_value: Option<u32>,
        record: Record,
    ) {
        let cpu_event = CpuEvent {
            segment,
            clk,
            pc,
            instruction,
            a,
            a_record: record.a,
            b,
            b_record: record.b,
            c,
            c_record: record.c,
            memory: memory_store_value,
            memory_record: record.memory,
        };
        self.segment.cpu_events.push(cpu_event);
    }

    /// Emit an ALU event.
    fn emit_alu(&mut self, clk: u32, opcode: Opcode, a: u32, b: u32, c: u32) {
        let event = AluEvent {
            clk,
            opcode,
            a,
            b,
            c,
        };
        match opcode {
            Opcode::ADD => {
                self.segment.add_events.push(event);
            }
            Opcode::SUB => {
                self.segment.sub_events.push(event);
            }
            Opcode::XOR | Opcode::OR | Opcode::AND => {
                self.segment.bitwise_events.push(event);
            }
            Opcode::SLL => {
                self.segment.shift_left_events.push(event);
            }
            Opcode::SRL | Opcode::SRA => {
                self.segment.shift_right_events.push(event);
            }
            Opcode::SLT | Opcode::SLTU => {
                self.segment.lt_events.push(event);
            }
            Opcode::MUL | Opcode::MULHU | Opcode::MULHSU | Opcode::MULH => {
                self.segment.add_events.push(event);
            }
            Opcode::DIVU | Opcode::REMU | Opcode::DIV | Opcode::REM => {
                self.segment.divrem_events.push(event);
            }
            _ => {}
        }
    }

    /// Fetch the destination register and input operand values for an ALU instruction.
    #[inline]
    fn alu_rr(&mut self, instruction: Instruction) -> (Register, u32, u32) {
        if !instruction.imm_c {
            let (rd, rs1, rs2) = instruction.r_type();
            let (rd, b, c) = (
                rd,
                self.rr(rs1, AccessPosition::B),
                self.rr(rs2, AccessPosition::C),
            );
            (rd, b, c)
        } else if !instruction.imm_b && instruction.imm_c {
            let (rd, rs1, imm) = instruction.i_type();
            let (rd, b, c) = (rd, self.rr(rs1, AccessPosition::B), imm);
            (rd, b, c)
        } else {
            assert!(instruction.imm_b && instruction.imm_c);
            let (rd, b, c) = (
                Register::from_u32(instruction.op_a),
                instruction.op_b,
                instruction.op_c,
            );
            (rd, b, c)
        }
    }

    /// Set the destination register with the result and emit an ALU event.
    #[inline]
    fn alu_rw(&mut self, instruction: Instruction, rd: Register, a: u32, b: u32, c: u32) {
        self.rw(rd, a);
        self.emit_alu(self.clk, instruction.opcode, a, b, c);
    }

    /// Fetch the input operand values for a load instruction.
    #[inline]
    fn load_rr(&mut self, instruction: Instruction) -> (Register, u32, u32, u32, u32) {
        let (rd, rs1, imm) = instruction.i_type();
        let (b, c) = (self.rr(rs1, AccessPosition::B), imm);
        let addr = b.wrapping_add(c);
        let memory_value = self.mr(self.align(addr), AccessPosition::Memory);
        (rd, b, c, addr, memory_value)
    }

    /// Fetch the input operand values for a store instruction.
    #[inline]
    fn store_rr(&mut self, instruction: Instruction) -> (u32, u32, u32, u32, u32) {
        let (rs1, rs2, imm) = instruction.s_type();
        let (a, b, c) = (
            self.rr(rs1, AccessPosition::A),
            self.rr(rs2, AccessPosition::B),
            imm,
        );
        let addr = b.wrapping_add(c);
        let memory_value = self.mr(self.align(addr), AccessPosition::Memory);
        (a, b, c, addr, memory_value)
    }

    /// Fetch the input operand values for a branch instruction.
    #[inline]
    fn branch_rr(&mut self, instruction: Instruction) -> (u32, u32, u32) {
        let (rs1, rs2, imm) = instruction.b_type();
        let (a, b, c) = (
            self.rr(rs1, AccessPosition::A),
            self.rr(rs2, AccessPosition::B),
            imm,
        );
        (a, b, c)
    }

    /// Fetch the instruction at the current program counter.
    fn fetch(&self) -> Instruction {
        let idx = ((self.pc - self.program.pc_base) / 4) as usize;
        return self.program.instructions[idx];
    }

    /// Execute the given instruction over the current state of the runtime.
    fn execute(&mut self, instruction: Instruction) {
        let pc = self.pc;
        let mut next_pc = self.pc.wrapping_add(4);

        let rd: Register;
        let (a, b, c): (u32, u32, u32);
        let (addr, memory_read_value): (u32, u32);
        let mut memory_store_value: Option<u32> = None;

        self.record = Record::default();

        match instruction.opcode {
            // Arithmetic instructions.
            Opcode::ADD => {
                (rd, b, c) = self.alu_rr(instruction);
                a = b.wrapping_add(c);
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::SUB => {
                (rd, b, c) = self.alu_rr(instruction);
                a = b.wrapping_sub(c);
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::XOR => {
                (rd, b, c) = self.alu_rr(instruction);
                a = b ^ c;
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::OR => {
                (rd, b, c) = self.alu_rr(instruction);
                a = b | c;
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::AND => {
                (rd, b, c) = self.alu_rr(instruction);
                a = b & c;
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::SLL => {
                (rd, b, c) = self.alu_rr(instruction);
                a = b.wrapping_shl(c);
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::SRL => {
                (rd, b, c) = self.alu_rr(instruction);
                a = b.wrapping_shr(c);
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::SRA => {
                (rd, b, c) = self.alu_rr(instruction);
                a = (b as i32).wrapping_shr(c) as u32;
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::SLT => {
                (rd, b, c) = self.alu_rr(instruction);
                a = if (b as i32) < (c as i32) { 1 } else { 0 };
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::SLTU => {
                (rd, b, c) = self.alu_rr(instruction);
                a = if b < c { 1 } else { 0 };
                self.alu_rw(instruction, rd, a, b, c);
            }

            // Load instructions.
            Opcode::LB => {
                (rd, b, c, addr, memory_read_value) = self.load_rr(instruction);
                let value = (memory_read_value).to_le_bytes()[(addr % 4) as usize];
                a = ((value as i8) as i32) as u32;
                memory_store_value = Some(memory_read_value);
                self.rw(rd, a);
            }
            Opcode::LH => {
                (rd, b, c, addr, memory_read_value) = self.load_rr(instruction);
                assert_eq!(addr % 2, 0, "addr is not aligned");
                let value = match addr % 4 {
                    0 => memory_read_value & 0x0000FFFF,
                    1 => memory_read_value & 0xFFFF0000,
                    _ => unreachable!(),
                };
                a = ((value as i16) as i32) as u32;
                memory_store_value = Some(memory_read_value);
                self.rw(rd, a);
            }
            Opcode::LW => {
                (rd, b, c, addr, memory_read_value) = self.load_rr(instruction);
                assert_eq!(addr % 4, 0, "addr is not aligned");
                a = memory_read_value;
                memory_store_value = Some(memory_read_value);
                self.rw(rd, a);
            }
            Opcode::LBU => {
                (rd, b, c, addr, memory_read_value) = self.load_rr(instruction);
                let value = (memory_read_value).to_le_bytes()[(addr % 4) as usize];
                a = (value as u8) as u32;
                memory_store_value = Some(memory_read_value);
                self.rw(rd, a);
            }
            Opcode::LHU => {
                (rd, b, c, addr, memory_read_value) = self.load_rr(instruction);
                assert_eq!(addr % 2, 0, "addr is not aligned");
                let value = if addr % 4 == 0 {
                    memory_read_value & 0x0000FFFF
                } else {
                    memory_read_value & 0xFFFF0000
                };
                a = (value as u16) as u32;
                memory_store_value = Some(memory_read_value);
                self.rw(rd, a);
            }

            // Store instructions.
            Opcode::SB => {
                (a, b, c, addr, memory_read_value) = self.store_rr(instruction);
                let value = match addr % 4 {
                    0 => (a & 0x000000FF) + (memory_read_value & 0xFFFFFF00),
                    1 => (a & 0x000000FF) << 8 + (memory_read_value & 0xFFFF00FF),
                    2 => (a & 0x000000FF) << 16 + (memory_read_value & 0xFF00FFFF),
                    3 => (a & 0x000000FF) << 24 + (memory_read_value & 0x00FFFFFF),
                    _ => unreachable!(),
                };
                memory_store_value = Some(value);
                self.mw(self.align(addr), value, AccessPosition::Memory);
            }
            Opcode::SH => {
                (a, b, c, addr, memory_read_value) = self.store_rr(instruction);
                assert_eq!(addr % 2, 0, "addr is not aligned");
                let value = match addr % 2 {
                    0 => (memory_read_value & 0xFFFF0000) + (a & 0x0000FFFF),
                    1 => (memory_read_value & 0x0000FFFF) + (a & 0x0000FFFF) << 16,
                    _ => unreachable!(),
                };
                memory_store_value = Some(value);
                self.mw(self.align(addr), value, AccessPosition::Memory);
            }
            Opcode::SW => {
                (a, b, c, addr, _) = self.store_rr(instruction);
                assert_eq!(addr % 4, 0, "addr is not aligned");
                let value = a;
                memory_store_value = Some(value);
                self.mw(self.align(addr), value, AccessPosition::Memory);
            }

            // B-type instructions.
            Opcode::BEQ => {
                (a, b, c) = self.branch_rr(instruction);
                if a == b {
                    next_pc = self.pc.wrapping_add(c);
                }
            }
            Opcode::BNE => {
                (a, b, c) = self.branch_rr(instruction);
                if a != b {
                    next_pc = self.pc.wrapping_add(c);
                }
            }
            Opcode::BLT => {
                (a, b, c) = self.branch_rr(instruction);
                if (a as i32) < (b as i32) {
                    next_pc = self.pc.wrapping_add(c);
                }
            }
            Opcode::BGE => {
                (a, b, c) = self.branch_rr(instruction);
                if (a as i32) >= (b as i32) {
                    next_pc = self.pc.wrapping_add(c);
                }
            }
            Opcode::BLTU => {
                (a, b, c) = self.branch_rr(instruction);
                if a < b {
                    next_pc = self.pc.wrapping_add(c);
                }
            }
            Opcode::BGEU => {
                (a, b, c) = self.branch_rr(instruction);
                if a >= b {
                    next_pc = self.pc.wrapping_add(c);
                }
            }

            // Jump instructions.
            Opcode::JAL => {
                let (rd, imm) = instruction.j_type();
                (b, c) = (imm, 0);
                a = self.pc + 4;
                self.rw(rd, a);
                next_pc = self.pc.wrapping_add(imm);
            }
            Opcode::JALR => {
                let (rd, rs1, imm) = instruction.i_type();
                (b, c) = (self.rr(rs1, AccessPosition::B), imm);
                a = self.pc + 4;
                self.rw(rd, a);
                next_pc = b.wrapping_add(c);
            }

            // Upper immediate instructions.
            Opcode::AUIPC => {
                let (rd, imm) = instruction.u_type();
                (b, c) = (imm, imm);
                a = self.pc.wrapping_add(b);
                self.rw(rd, a);
            }

            // System instructions.
            Opcode::ECALL => {
                let t0 = Register::X5;
                let a0 = Register::X10;
                let syscall_id = self.register(t0);
                let syscall = Syscall::from_u32(syscall_id);
                match syscall {
                    Syscall::HALT => {
                        a = self.register(a0);
                        (b, c) = (self.rr(t0, AccessPosition::B), 0);
                        next_pc = 0;
                        self.rw(a0, a);
                    }
                    Syscall::LWA => {
                        let witness = self.witness.pop().expect("witness stream is empty");
                        println!("witness {}", witness);
                        (a, b, c) = (witness, self.rr(t0, AccessPosition::B), 0);
                        self.rw(a0, a);
                    }
                    Syscall::SHA_EXTEND => {
                        // The number of cycles it takes to perform this precompile.
                        const NB_SHA_EXTEND_CYCLES: u32 = 48 * 20;

                        // Temporarily set the clock to the number of cycles it takes to perform
                        // this precompile as reading `w_ptr` happens on this clock.
                        self.clk += NB_SHA_EXTEND_CYCLES;

                        // Read `w_ptr` from register a0 or x5.
                        let w_ptr = self.register(a0);
                        let mut w = Vec::new();
                        for i in 0..64 {
                            w.push(self.word(w_ptr + i * 4));
                        }

                        // Set the CPU table values with some dummy values.
                        (a, b, c) = (w_ptr, self.rr(t0, AccessPosition::B), 0);
                        self.rw(a0, a);

                        // We'll save the current record and restore it later so that the CPU
                        // event gets emitted correctly.
                        let t = self.record;

                        // Set the clock back to the original value and begin executing the
                        // precompile.
                        self.clk -= NB_SHA_EXTEND_CYCLES;
                        let saved_clk = self.clk;
                        let saved_w_ptr = w_ptr;
                        let saved_w = w.clone();
                        let mut w_i_minus_15_records = Vec::new();
                        let mut w_i_minus_2_records = Vec::new();
                        let mut w_i_minus_16_records = Vec::new();
                        let mut w_i_minus_7_records = Vec::new();
                        let mut w_i_records = Vec::new();
                        for i in 16..64 {
                            // Read w[i-15].
                            let w_i_minus_15 =
                                self.mr(w_ptr + (i - 15) * 4, AccessPosition::Memory);
                            w_i_minus_15_records.push(self.record.memory);
                            self.clk += 4;

                            // Compute `s0`.
                            let s0 = w_i_minus_15.rotate_right(7)
                                ^ w_i_minus_15.rotate_right(18)
                                ^ (w_i_minus_15 >> 3);

                            // Read w[i-2].
                            let w_i_minus_2 = self.mr(w_ptr + (i - 2) * 4, AccessPosition::Memory);
                            w_i_minus_2_records.push(self.record.memory);
                            self.clk += 4;

                            // Compute `s1`.
                            let s1 = w_i_minus_2.rotate_right(17)
                                ^ w_i_minus_2.rotate_right(19)
                                ^ (w_i_minus_2 >> 10);

                            // Read w[i-16].
                            let w_i_minus_16 =
                                self.mr(w_ptr + (i - 16) * 4, AccessPosition::Memory);
                            w_i_minus_16_records.push(self.record.memory);
                            self.clk += 4;

                            // Read w[i-7].
                            let w_i_minus_7 = self.mr(w_ptr + (i - 7) * 4, AccessPosition::Memory);
                            w_i_minus_7_records.push(self.record.memory);
                            self.clk += 4;

                            // Compute `w_i`.
                            let w_i = s1
                                .wrapping_add(w_i_minus_16)
                                .wrapping_add(s0)
                                .wrapping_add(w_i_minus_7);

                            // Write w[i].
                            self.mr(w_ptr + i * 4, AccessPosition::Memory);
                            self.mw(w_ptr + i * 4, w_i, AccessPosition::Memory);
                            w_i_records.push(self.record.memory);
                            self.clk += 4;
                        }

                        // Push the SHA extend event.
                        self.segment.sha_extend_events.push(ShaExtendEvent {
                            clk: saved_clk,
                            w_ptr: saved_w_ptr,
                            w: saved_w.try_into().unwrap(),
                            w_i_minus_15_records: w_i_minus_15_records.try_into().unwrap(),
                            w_i_minus_2_records: w_i_minus_2_records.try_into().unwrap(),
                            w_i_minus_16_records: w_i_minus_16_records.try_into().unwrap(),
                            w_i_minus_7_records: w_i_minus_7_records.try_into().unwrap(),
                            w_i_records: w_i_records.try_into().unwrap(),
                        });

                        // Restore the original record.
                        self.record = t;
                    }
                    Syscall::SHA_COMPRESS => {
                        // The number of cycles it takes to perform this precompile.
                        const NB_SHA_COMPRESS_CYCLES: u32 = 8 * 4 + 64 * 4 + 8 * 4;

                        // Temporarily set the clock to the number of cycles it takes to perform
                        // this precompile as reading `w_ptr` happens on this clock.
                        self.clk += NB_SHA_COMPRESS_CYCLES;

                        // Read `w_ptr` from register a0 or x5.
                        let w_ptr = self.register(a0);
                        let mut w = Vec::new();
                        for i in 0..64 {
                            w.push(self.word(w_ptr + i * 4));
                        }

                        // Set the CPU table values with some dummy values.
                        (a, b, c) = (w_ptr, self.rr(t0, AccessPosition::B), 0);
                        self.rw(a0, a);

                        // We'll save the current record and restore it later so that the CPU
                        // event gets emitted correctly.
                        let t = self.record;

                        // Set the clock back to the original value and begin executing the
                        // precompile.
                        self.clk -= NB_SHA_COMPRESS_CYCLES;
                        let saved_clk = self.clk;
                        let saved_w_ptr = w_ptr;
                        let saved_w = w.clone();
                        let mut h_read_records = Vec::new();
                        let mut w_i_read_records = Vec::new();
                        let mut h_write_records = Vec::new();

                        // Execute the "initialize" phase.
                        const H_START_IDX: u32 = 64;
                        let mut hx = [0u32; 8];
                        for i in 0..8 {
                            hx[i] = self
                                .mr(w_ptr + (H_START_IDX + i as u32) * 4, AccessPosition::Memory);
                            h_read_records.push(self.record.memory);
                            self.clk += 4;
                        }

                        // Execute the "compress" phase.
                        let mut a = hx[0];
                        let mut b = hx[1];
                        let mut c = hx[2];
                        let mut d = hx[3];
                        let mut e = hx[4];
                        let mut f = hx[5];
                        let mut g = hx[6];
                        let mut h = hx[7];
                        for i in 0..64 {
                            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                            let ch = (e & f) ^ (!e & g);
                            let w_i = self.mr(w_ptr + i * 4, AccessPosition::Memory);
                            w_i_read_records.push(self.record.memory);
                            let temp1 = h
                                .wrapping_add(s1)
                                .wrapping_add(ch)
                                .wrapping_add(SHA_COMPRESS_K[i as usize])
                                .wrapping_add(w_i);
                            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                            let maj = (a & b) ^ (a & c) ^ (b & c);
                            let temp2 = s0.wrapping_add(maj);

                            h = g;
                            g = f;
                            f = e;
                            e = d + temp1;
                            d = c;
                            c = b;
                            b = a;
                            a = temp1 + temp2;

                            self.clk += 4;
                        }

                        // Execute the "finalize" phase.
                        let v = [a, b, c, d, e, f, g, h];
                        for i in 0..8 {
                            self.mr(w_ptr + (H_START_IDX + i as u32) * 4, AccessPosition::Memory);
                            self.mw(
                                w_ptr + (H_START_IDX + i as u32) * 4,
                                hx[i] + v[i],
                                AccessPosition::Memory,
                            );
                            h_write_records.push(self.record.memory);
                            self.clk += 4;
                        }

                        // Push the SHA extend event.
                        self.segment.sha_compress_events.push(ShaCompressEvent {
                            clk: saved_clk,
                            w_and_h_ptr: saved_w_ptr,
                            w: saved_w.try_into().unwrap(),
                            h: hx,
                            h_read_records: h_read_records.try_into().unwrap(),
                            w_i_read_records: w_i_read_records.try_into().unwrap(),
                            h_write_records: h_write_records.try_into().unwrap(),
                        });

                        // Restore the original record.
                        self.record = t;
                    }
                }
            }

            Opcode::EBREAK => {
                todo!()
            }

            // Multiply instructions.
            Opcode::MUL => {
                (rd, b, c) = self.alu_rr(instruction);
                a = b.wrapping_mul(c);
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::MULH => {
                (rd, b, c) = self.alu_rr(instruction);
                a = (((b as i32) as i64).wrapping_mul((c as i32) as i64) >> 32) as u32;
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::MULHU => {
                (rd, b, c) = self.alu_rr(instruction);
                a = ((b as u64).wrapping_mul(c as u64) >> 32) as u32;
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::MULHSU => {
                (rd, b, c) = self.alu_rr(instruction);
                a = (((b as i32) as i64).wrapping_mul(c as i64) >> 32) as u32;
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::DIV => {
                (rd, b, c) = self.alu_rr(instruction);
                if c == 0 {
                    a = u32::MAX;
                } else {
                    a = (b as i32).wrapping_div(c as i32) as u32;
                }
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::DIVU => {
                (rd, b, c) = self.alu_rr(instruction);
                if c == 0 {
                    a = u32::MAX;
                } else {
                    a = b.wrapping_div(c);
                }
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::REM => {
                (rd, b, c) = self.alu_rr(instruction);
                if c == 0 {
                    a = b;
                } else {
                    a = (b as i32).wrapping_rem(c as i32) as u32;
                }
                self.alu_rw(instruction, rd, a, b, c);
            }
            Opcode::REMU => {
                (rd, b, c) = self.alu_rr(instruction);
                if c == 0 {
                    a = b;
                } else {
                    a = b.wrapping_rem(c);
                }
                self.alu_rw(instruction, rd, a, b, c);
            }

            Opcode::UNIMP => {
                // See https://github.com/riscv-non-isa/riscv-asm-manual/blob/master/riscv-asm.md#instruction-aliases
                panic!("UNIMP encountered, we should never get here.");
            }
        }

        // Update the program counter.
        self.pc = next_pc;

        // Emit the CPU event for this cycle.
        self.emit_cpu(
            self.current_segment(),
            self.clk,
            pc,
            instruction,
            a,
            b,
            c,
            memory_store_value,
            self.record,
        );
    }

    /// Execute the program.
    pub fn run(&mut self) {
        // First load the memory image into the memory table.
        for (addr, value) in self.program.memory_image.iter() {
            self.memory.insert(*addr, *value);
            self.memory_access.insert(*addr, (0, 0));
        }

        self.clk += 1;
        while self.pc.wrapping_sub(self.program.pc_base)
            < (self.program.instructions.len() * 4) as u32
        {
            // Fetch the instruction at the current program counter.
            let instruction = self.fetch();

            let width = 12;
            log::debug!(
                "clk={} [pc=0x{:x?}] {:<width$?} |         x0={:<width$} x1={:<width$} x2={:<width$} x3={:<width$} x4={:<width$} x5={:<width$} x6={:<width$} x7={:<width$} x8={:<width$} x9={:<width$} x10={:<width$} x11={:<width$}",
                self.global_clk / 4,
                self.pc,
                instruction,
                self.register(Register::X0),
                self.register(Register::X1),
                self.register(Register::X2),
                self.register(Register::X3),
                self.register(Register::X4),
                self.register(Register::X5),
                self.register(Register::X6),
                self.register(Register::X7),
                self.register(Register::X8),
                self.register(Register::X9),
                self.register(Register::X10),
                self.register(Register::X11)
            );

            // Execute the instruction.
            self.execute(instruction);

            // Increment the clock.
            self.global_clk += 4;
            self.clk += 4;

            if self.clk % self.segment_size == 0 {
                self.segments.push(self.segment.clone());
                // Set up new segment
                self.segment = Segment::default();
                self.segment.index = self.segments.len() as u32 + 1;
                self.segment.program = self.program.clone();
                self.clk = 1;
            }
        }

        // Push the last segment.
        // TODO: edge case where the last segment is empty.
        self.segments.push(self.segment.clone());

        // Right now we only do 1 segment.
        assert_eq!(self.segments.len(), 1);

        // Call postprocess to set up all variables needed for global accounts, like memory
        // argument or any other deferred tables.
        self.postprocess();
    }

    fn postprocess(&mut self) {
        let mut program_memory_used = BTreeMap::new();
        for (key, value) in &self.program.memory_image {
            // By default we assume that the program_memory is used.
            program_memory_used.insert((*key, *value), 1);
        }

        let mut first_memory_record = Vec::new();
        let mut last_memory_record = Vec::new();

        for (addr, value) in &self.memory {
            let (segment, timestamp) = self.memory_access.get(&addr).unwrap().clone();
            if segment == 0 && timestamp == 0 {
                // This means that we never accessed this memory location throughout our entire program.
                // The only way this can happen is if this was in the program memory image.
                // We mark this (addr, value) as not used in the `program_memory_used` map.
                program_memory_used.insert((*addr, *value), 0);
                continue;
            }
            // If the memory addr was accessed, we only add it to "first_memory_record" if it was
            // not in the program_memory_image, otherwise we'll add to the memory argument from
            // the program_memory_image table.
            if !self.program.memory_image.contains_key(addr) {
                first_memory_record.push((
                    *addr,
                    MemoryRecord {
                        value: 0,
                        segment: 0,
                        timestamp: 0,
                    },
                    1,
                ));
            }

            last_memory_record.push((
                *addr,
                MemoryRecord {
                    value: *value,
                    segment,
                    timestamp,
                },
                1,
            ))
        }

        let mut program_memory_record = program_memory_used
            .iter()
            .map(|(&(addr, value), &used)| {
                (
                    addr,
                    MemoryRecord {
                        value: value,
                        segment: 0,
                        timestamp: 0,
                    },
                    used,
                )
            })
            .collect::<Vec<(u32, MemoryRecord, u32)>>();
        program_memory_record.sort_by_key(|&(addr, _, _)| addr);

        self.global_segment.first_memory_record = first_memory_record;
        self.global_segment.last_memory_record = last_memory_record;
        self.global_segment.program_memory_record = program_memory_record;
    }
}

#[cfg(test)]
pub mod tests {

    use log::debug;

    use crate::runtime::Register;

    use super::{Instruction, Opcode, Program, Runtime};

    pub fn simple_program() -> Program {
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 0, 37, false, true),
            Instruction::new(Opcode::ADD, 31, 30, 29, false, false),
        ];
        Program::new(instructions, 0, 0)
    }

    pub fn fibonacci_program() -> Program {
        Program::from_elf("../programs/fib_malloc.s")
    }

    pub fn ecall_lwa_program() -> Program {
        let instructions = vec![
            Instruction::new(Opcode::ADD, 5, 0, 101, false, true),
            Instruction::new(Opcode::ECALL, 10, 5, 0, false, true),
        ];
        Program::new(instructions, 0, 0)
    }

    #[test]
    fn test_simple_program_run() {
        let program = simple_program();
        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 42);
    }

    #[test]
    fn test_fibonacci_run() {
        if env_logger::try_init().is_err() {
            debug!("Logger already initialized")
        }
        let program = fibonacci_program();
        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.registers()[Register::X10 as usize], 144);
    }

    #[test]
    fn test_add() {
        // main:
        //     addi x29, x0, 5
        //     addi x30, x0, 37
        //     add x31, x30, x29
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 0, 37, false, true),
            Instruction::new(Opcode::ADD, 31, 30, 29, false, false),
        ];
        let program = Program::new(instructions, 0, 0);
        let mut runtime = Runtime::new(program);
        runtime.run();

        assert_eq!(runtime.register(Register::X31), 42);
    }

    #[test]
    fn test_sub() {
        //     addi x29, x0, 5
        //     addi x30, x0, 37
        //     sub x31, x30, x29
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 0, 37, false, true),
            Instruction::new(Opcode::SUB, 31, 30, 29, false, false),
        ];
        let program = Program::new(instructions, 0, 0);

        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 32);
    }

    #[test]
    fn test_xor() {
        //     addi x29, x0, 5
        //     addi x30, x0, 37
        //     xor x31, x30, x29
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 0, 37, false, true),
            Instruction::new(Opcode::XOR, 31, 30, 29, false, false),
        ];
        let program = Program::new(instructions, 0, 0);

        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 32);
    }

    #[test]
    fn test_or() {
        //     addi x29, x0, 5
        //     addi x30, x0, 37
        //     or x31, x30, x29
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 0, 37, false, true),
            Instruction::new(Opcode::OR, 31, 30, 29, false, false),
        ];
        let program = Program::new(instructions, 0, 0);

        let mut runtime = Runtime::new(program);

        runtime.run();
        assert_eq!(runtime.register(Register::X31), 37);
    }

    #[test]
    fn test_and() {
        //     addi x29, x0, 5
        //     addi x30, x0, 37
        //     and x31, x30, x29
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 0, 37, false, true),
            Instruction::new(Opcode::AND, 31, 30, 29, false, false),
        ];
        let program = Program::new(instructions, 0, 0);

        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 5);
    }

    #[test]
    fn test_sll() {
        //     addi x29, x0, 5
        //     addi x30, x0, 37
        //     sll x31, x30, x29
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 0, 37, false, true),
            Instruction::new(Opcode::SLL, 31, 30, 29, false, false),
        ];
        let program = Program::new(instructions, 0, 0);

        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 1184);
    }

    #[test]
    fn test_srl() {
        //     addi x29, x0, 5
        //     addi x30, x0, 37
        //     srl x31, x30, x29
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 0, 37, false, true),
            Instruction::new(Opcode::SRL, 31, 30, 29, false, false),
        ];
        let program = Program::new(instructions, 0, 0);

        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 1);
    }

    #[test]
    fn test_sra() {
        //     addi x29, x0, 5
        //     addi x30, x0, 37
        //     sra x31, x30, x29
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 0, 37, false, true),
            Instruction::new(Opcode::SRA, 31, 30, 29, false, false),
        ];
        let program = Program::new(instructions, 0, 0);

        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 1);
    }

    #[test]
    fn test_slt() {
        //     addi x29, x0, 5
        //     addi x30, x0, 37
        //     slt x31, x30, x29
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 0, 37, false, true),
            Instruction::new(Opcode::SLT, 31, 30, 29, false, false),
        ];
        let program = Program::new(instructions, 0, 0);

        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 0);
    }

    #[test]
    fn test_sltu() {
        //     addi x29, x0, 5
        //     addi x30, x0, 37
        //     sltu x31, x30, x29
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 0, 37, false, true),
            Instruction::new(Opcode::SLTU, 31, 30, 29, false, false),
        ];
        let program = Program::new(instructions, 0, 0);

        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 0);
    }

    #[test]
    fn test_addi() {
        //     addi x29, x0, 5
        //     addi x30, x29, 37
        //     addi x31, x30, 42
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 29, 37, false, true),
            Instruction::new(Opcode::ADD, 31, 30, 42, false, true),
        ];
        let program = Program::new(instructions, 0, 0);

        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 84);
    }

    #[test]
    fn test_addi_negative() {
        //     addi x29, x0, 5
        //     addi x30, x29, -1
        //     addi x31, x30, 4
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 29, 0xffffffff, false, true),
            Instruction::new(Opcode::ADD, 31, 30, 4, false, true),
        ];
        let program = Program::new(instructions, 0, 0);
        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 5 - 1 + 4);
    }

    #[test]
    fn test_xori() {
        //     addi x29, x0, 5
        //     xori x30, x29, 37
        //     xori x31, x30, 42
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::XOR, 30, 29, 37, false, true),
            Instruction::new(Opcode::XOR, 31, 30, 42, false, true),
        ];
        let program = Program::new(instructions, 0, 0);
        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 10);
    }

    #[test]
    fn test_ori() {
        //     addi x29, x0, 5
        //     ori x30, x29, 37
        //     ori x31, x30, 42
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::OR, 30, 29, 37, false, true),
            Instruction::new(Opcode::OR, 31, 30, 42, false, true),
        ];
        let program = Program::new(instructions, 0, 0);
        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 47);
    }

    #[test]
    fn test_andi() {
        //     addi x29, x0, 5
        //     andi x30, x29, 37
        //     andi x31, x30, 42
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::AND, 30, 29, 37, false, true),
            Instruction::new(Opcode::AND, 31, 30, 42, false, true),
        ];
        let program = Program::new(instructions, 0, 0);
        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 0);
    }

    #[test]
    fn test_slli() {
        //     addi x29, x0, 5
        //     slli x31, x29, 37
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::SLL, 31, 29, 4, false, true),
        ];
        let program = Program::new(instructions, 0, 0);
        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 80);
    }

    #[test]
    fn test_srli() {
        //    addi x29, x0, 5
        //    srli x31, x29, 37
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 42, false, true),
            Instruction::new(Opcode::SRL, 31, 29, 4, false, true),
        ];
        let program = Program::new(instructions, 0, 0);
        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 2);
    }

    #[test]
    fn test_srai() {
        //   addi x29, x0, 5
        //   srai x31, x29, 37
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 42, false, true),
            Instruction::new(Opcode::SRA, 31, 29, 4, false, true),
        ];
        let program = Program::new(instructions, 0, 0);
        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 2);
    }

    #[test]
    fn test_slti() {
        //   addi x29, x0, 5
        //   slti x31, x29, 37
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 42, false, true),
            Instruction::new(Opcode::SLT, 31, 29, 37, false, true),
        ];
        let program = Program::new(instructions, 0, 0);
        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 0);
    }

    #[test]
    fn test_sltiu() {
        //   addi x29, x0, 5
        //   sltiu x31, x29, 37
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 42, false, true),
            Instruction::new(Opcode::SLTU, 31, 29, 37, false, true),
        ];
        let program = Program::new(instructions, 0, 0);
        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.register(Register::X31), 0);
    }

    #[test]
    fn test_jalr() {
        //   addi x11, x11, 100
        //   jalr x5, x11, 8
        //
        // `JALR rd offset(rs)` reads the value at rs, adds offset to it and uses it as the
        // destination address. It then stores the address of the next instruction in rd in case
        // we'd want to come back here.

        let instructions = vec![
            Instruction::new(Opcode::ADD, 11, 11, 100, false, true),
            Instruction::new(Opcode::JALR, 5, 11, 8, false, true),
        ];
        let program = Program::new(instructions, 0, 0);
        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.registers()[Register::X5 as usize], 8);
        assert_eq!(runtime.registers()[Register::X11 as usize], 100);
        assert_eq!(runtime.pc, 108);
    }

    fn simple_op_code_test(opcode: Opcode, expected: u32, a: u32, b: u32) {
        let instructions = vec![
            Instruction::new(Opcode::ADD, 10, 0, a, false, true),
            Instruction::new(Opcode::ADD, 11, 0, b, false, true),
            Instruction::new(opcode, 12, 10, 11, false, false),
        ];
        let program = Program::new(instructions, 0, 0);
        let mut runtime = Runtime::new(program);
        runtime.run();
        assert_eq!(runtime.registers()[Register::X12 as usize], expected);
    }

    #[test]
    fn multiplication_tests() {
        simple_op_code_test(Opcode::MULHU, 0x00000000, 0x00000000, 0x00000000);
        simple_op_code_test(Opcode::MULHU, 0x00000000, 0x00000001, 0x00000001);
        simple_op_code_test(Opcode::MULHU, 0x00000000, 0x00000003, 0x00000007);
        simple_op_code_test(Opcode::MULHU, 0x00000000, 0x00000000, 0xffff8000);
        simple_op_code_test(Opcode::MULHU, 0x00000000, 0x80000000, 0x00000000);
        simple_op_code_test(Opcode::MULHU, 0x7fffc000, 0x80000000, 0xffff8000);
        simple_op_code_test(Opcode::MULHU, 0x0001fefe, 0xaaaaaaab, 0x0002fe7d);
        simple_op_code_test(Opcode::MULHU, 0x0001fefe, 0x0002fe7d, 0xaaaaaaab);
        simple_op_code_test(Opcode::MULHU, 0xfe010000, 0xff000000, 0xff000000);
        simple_op_code_test(Opcode::MULHU, 0xfffffffe, 0xffffffff, 0xffffffff);
        simple_op_code_test(Opcode::MULHU, 0x00000000, 0xffffffff, 0x00000001);
        simple_op_code_test(Opcode::MULHU, 0x00000000, 0x00000001, 0xffffffff);

        simple_op_code_test(Opcode::MULHSU, 0x00000000, 0x00000000, 0x00000000);
        simple_op_code_test(Opcode::MULHSU, 0x00000000, 0x00000001, 0x00000001);
        simple_op_code_test(Opcode::MULHSU, 0x00000000, 0x00000003, 0x00000007);
        simple_op_code_test(Opcode::MULHSU, 0x00000000, 0x00000000, 0xffff8000);
        simple_op_code_test(Opcode::MULHSU, 0x00000000, 0x80000000, 0x00000000);
        simple_op_code_test(Opcode::MULHSU, 0x80004000, 0x80000000, 0xffff8000);
        simple_op_code_test(Opcode::MULHSU, 0xffff0081, 0xaaaaaaab, 0x0002fe7d);
        simple_op_code_test(Opcode::MULHSU, 0x0001fefe, 0x0002fe7d, 0xaaaaaaab);
        simple_op_code_test(Opcode::MULHSU, 0xff010000, 0xff000000, 0xff000000);
        simple_op_code_test(Opcode::MULHSU, 0xffffffff, 0xffffffff, 0xffffffff);
        simple_op_code_test(Opcode::MULHSU, 0xffffffff, 0xffffffff, 0x00000001);
        simple_op_code_test(Opcode::MULHSU, 0x00000000, 0x00000001, 0xffffffff);

        simple_op_code_test(Opcode::MULH, 0x00000000, 0x00000000, 0x00000000);
        simple_op_code_test(Opcode::MULH, 0x00000000, 0x00000001, 0x00000001);
        simple_op_code_test(Opcode::MULH, 0x00000000, 0x00000003, 0x00000007);
        simple_op_code_test(Opcode::MULH, 0x00000000, 0x00000000, 0xffff8000);
        simple_op_code_test(Opcode::MULH, 0x00000000, 0x80000000, 0x00000000);
        simple_op_code_test(Opcode::MULH, 0x00000000, 0x80000000, 0x00000000);
        simple_op_code_test(Opcode::MULH, 0xffff0081, 0xaaaaaaab, 0x0002fe7d);
        simple_op_code_test(Opcode::MULH, 0xffff0081, 0x0002fe7d, 0xaaaaaaab);
        simple_op_code_test(Opcode::MULH, 0x00010000, 0xff000000, 0xff000000);
        simple_op_code_test(Opcode::MULH, 0x00000000, 0xffffffff, 0xffffffff);
        simple_op_code_test(Opcode::MULH, 0xffffffff, 0xffffffff, 0x00000001);
        simple_op_code_test(Opcode::MULH, 0xffffffff, 0x00000001, 0xffffffff);

        simple_op_code_test(Opcode::MUL, 0x00001200, 0x00007e00, 0xb6db6db7);
        simple_op_code_test(Opcode::MUL, 0x00001240, 0x00007fc0, 0xb6db6db7);
        simple_op_code_test(Opcode::MUL, 0x00000000, 0x00000000, 0x00000000);
        simple_op_code_test(Opcode::MUL, 0x00000001, 0x00000001, 0x00000001);
        simple_op_code_test(Opcode::MUL, 0x00000015, 0x00000003, 0x00000007);
        simple_op_code_test(Opcode::MUL, 0x00000000, 0x00000000, 0xffff8000);
        simple_op_code_test(Opcode::MUL, 0x00000000, 0x80000000, 0x00000000);
        simple_op_code_test(Opcode::MUL, 0x00000000, 0x80000000, 0xffff8000);
        simple_op_code_test(Opcode::MUL, 0x0000ff7f, 0xaaaaaaab, 0x0002fe7d);
        simple_op_code_test(Opcode::MUL, 0x0000ff7f, 0x0002fe7d, 0xaaaaaaab);
        simple_op_code_test(Opcode::MUL, 0x00000000, 0xff000000, 0xff000000);
        simple_op_code_test(Opcode::MUL, 0x00000001, 0xffffffff, 0xffffffff);
        simple_op_code_test(Opcode::MUL, 0xffffffff, 0xffffffff, 0x00000001);
        simple_op_code_test(Opcode::MUL, 0xffffffff, 0x00000001, 0xffffffff);
    }

    fn neg(a: u32) -> u32 {
        u32::MAX - a + 1
    }

    #[test]
    fn division_tests() {
        simple_op_code_test(Opcode::DIVU, 3, 20, 6);
        simple_op_code_test(Opcode::DIVU, 715827879, u32::MAX - 20 + 1, 6);
        simple_op_code_test(Opcode::DIVU, 0, 20, u32::MAX - 6 + 1);
        simple_op_code_test(Opcode::DIVU, 0, u32::MAX - 20 + 1, u32::MAX - 6 + 1);

        simple_op_code_test(Opcode::DIVU, 1 << 31, 1 << 31, 1);
        simple_op_code_test(Opcode::DIVU, 0, 1 << 31, u32::MAX - 1 + 1);

        simple_op_code_test(Opcode::DIVU, u32::MAX, 1 << 31, 0);
        simple_op_code_test(Opcode::DIVU, u32::MAX, 1, 0);
        simple_op_code_test(Opcode::DIVU, u32::MAX, 0, 0);

        simple_op_code_test(Opcode::DIV, 3, 18, 6);
        simple_op_code_test(Opcode::DIV, neg(6), neg(24), 4);
        simple_op_code_test(Opcode::DIV, neg(2), 16, neg(8));
        simple_op_code_test(Opcode::DIV, neg(1), 0, 0);

        // Overflow cases
        simple_op_code_test(Opcode::DIV, 1 << 31, 1 << 31, neg(1));
        simple_op_code_test(Opcode::REM, 0, 1 << 31, neg(1));
    }

    #[test]
    fn remainder_tests() {
        simple_op_code_test(Opcode::REM, 7, 16, 9);
        simple_op_code_test(Opcode::REM, neg(4), neg(22), 6);
        simple_op_code_test(Opcode::REM, 1, 25, neg(3));
        simple_op_code_test(Opcode::REM, neg(2), neg(22), neg(4));
        simple_op_code_test(Opcode::REM, 0, 873, 1);
        simple_op_code_test(Opcode::REM, 0, 873, neg(1));
        simple_op_code_test(Opcode::REM, 5, 5, 0);
        simple_op_code_test(Opcode::REM, neg(5), neg(5), 0);
        simple_op_code_test(Opcode::REM, 0, 0, 0);

        simple_op_code_test(Opcode::REMU, 4, 18, 7);
        simple_op_code_test(Opcode::REMU, 6, neg(20), 11);
        simple_op_code_test(Opcode::REMU, 23, 23, neg(6));
        simple_op_code_test(Opcode::REMU, neg(21), neg(21), neg(11));
        simple_op_code_test(Opcode::REMU, 5, 5, 0);
        simple_op_code_test(Opcode::REMU, neg(1), neg(1), 0);
        simple_op_code_test(Opcode::REMU, 0, 0, 0);
    }

    #[test]
    fn shift_tests() {
        simple_op_code_test(Opcode::SLL, 0x00000001, 0x00000001, 0);
        simple_op_code_test(Opcode::SLL, 0x00000002, 0x00000001, 1);
        simple_op_code_test(Opcode::SLL, 0x00000080, 0x00000001, 7);
        simple_op_code_test(Opcode::SLL, 0x00004000, 0x00000001, 14);
        simple_op_code_test(Opcode::SLL, 0x80000000, 0x00000001, 31);
        simple_op_code_test(Opcode::SLL, 0xffffffff, 0xffffffff, 0);
        simple_op_code_test(Opcode::SLL, 0xfffffffe, 0xffffffff, 1);
        simple_op_code_test(Opcode::SLL, 0xffffff80, 0xffffffff, 7);
        simple_op_code_test(Opcode::SLL, 0xffffc000, 0xffffffff, 14);
        simple_op_code_test(Opcode::SLL, 0x80000000, 0xffffffff, 31);
        simple_op_code_test(Opcode::SLL, 0x21212121, 0x21212121, 0);
        simple_op_code_test(Opcode::SLL, 0x42424242, 0x21212121, 1);
        simple_op_code_test(Opcode::SLL, 0x90909080, 0x21212121, 7);
        simple_op_code_test(Opcode::SLL, 0x48484000, 0x21212121, 14);
        simple_op_code_test(Opcode::SLL, 0x80000000, 0x21212121, 31);
        simple_op_code_test(Opcode::SLL, 0x21212121, 0x21212121, 0xffffffe0);
        simple_op_code_test(Opcode::SLL, 0x42424242, 0x21212121, 0xffffffe1);
        simple_op_code_test(Opcode::SLL, 0x90909080, 0x21212121, 0xffffffe7);
        simple_op_code_test(Opcode::SLL, 0x48484000, 0x21212121, 0xffffffee);
        simple_op_code_test(Opcode::SLL, 0x00000000, 0x21212120, 0xffffffff);

        simple_op_code_test(Opcode::SRL, 0xffff8000, 0xffff8000, 0);
        simple_op_code_test(Opcode::SRL, 0x7fffc000, 0xffff8000, 1);
        simple_op_code_test(Opcode::SRL, 0x01ffff00, 0xffff8000, 7);
        simple_op_code_test(Opcode::SRL, 0x0003fffe, 0xffff8000, 14);
        simple_op_code_test(Opcode::SRL, 0x0001ffff, 0xffff8001, 15);
        simple_op_code_test(Opcode::SRL, 0xffffffff, 0xffffffff, 0);
        simple_op_code_test(Opcode::SRL, 0x7fffffff, 0xffffffff, 1);
        simple_op_code_test(Opcode::SRL, 0x01ffffff, 0xffffffff, 7);
        simple_op_code_test(Opcode::SRL, 0x0003ffff, 0xffffffff, 14);
        simple_op_code_test(Opcode::SRL, 0x00000001, 0xffffffff, 31);
        simple_op_code_test(Opcode::SRL, 0x21212121, 0x21212121, 0);
        simple_op_code_test(Opcode::SRL, 0x10909090, 0x21212121, 1);
        simple_op_code_test(Opcode::SRL, 0x00424242, 0x21212121, 7);
        simple_op_code_test(Opcode::SRL, 0x00008484, 0x21212121, 14);
        simple_op_code_test(Opcode::SRL, 0x00000000, 0x21212121, 31);
        simple_op_code_test(Opcode::SRL, 0x21212121, 0x21212121, 0xffffffe0);
        simple_op_code_test(Opcode::SRL, 0x10909090, 0x21212121, 0xffffffe1);
        simple_op_code_test(Opcode::SRL, 0x00424242, 0x21212121, 0xffffffe7);
        simple_op_code_test(Opcode::SRL, 0x00008484, 0x21212121, 0xffffffee);
        simple_op_code_test(Opcode::SRL, 0x00000000, 0x21212121, 0xffffffff);

        simple_op_code_test(Opcode::SRA, 0x00000000, 0x00000000, 0);
        simple_op_code_test(Opcode::SRA, 0xc0000000, 0x80000000, 1);
        simple_op_code_test(Opcode::SRA, 0xff000000, 0x80000000, 7);
        simple_op_code_test(Opcode::SRA, 0xfffe0000, 0x80000000, 14);
        simple_op_code_test(Opcode::SRA, 0xffffffff, 0x80000001, 31);
        simple_op_code_test(Opcode::SRA, 0x7fffffff, 0x7fffffff, 0);
        simple_op_code_test(Opcode::SRA, 0x3fffffff, 0x7fffffff, 1);
        simple_op_code_test(Opcode::SRA, 0x00ffffff, 0x7fffffff, 7);
        simple_op_code_test(Opcode::SRA, 0x0001ffff, 0x7fffffff, 14);
        simple_op_code_test(Opcode::SRA, 0x00000000, 0x7fffffff, 31);
        simple_op_code_test(Opcode::SRA, 0x81818181, 0x81818181, 0);
        simple_op_code_test(Opcode::SRA, 0xc0c0c0c0, 0x81818181, 1);
        simple_op_code_test(Opcode::SRA, 0xff030303, 0x81818181, 7);
        simple_op_code_test(Opcode::SRA, 0xfffe0606, 0x81818181, 14);
        simple_op_code_test(Opcode::SRA, 0xffffffff, 0x81818181, 31);
    }
}
