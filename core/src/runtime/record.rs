use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use super::program::Program;
use super::Opcode;
use crate::alu::AluEvent;
use crate::bytes::{ByteLookupEvent, ByteOpcode};
use crate::cpu::{CpuEvent, MemoryRecordEnum};
use crate::field::event::FieldEvent;
use crate::runtime::MemoryRecord;
use crate::syscall::precompiles::blake3::Blake3CompressInnerEvent;
use crate::syscall::precompiles::edwards::EdDecompressEvent;
use crate::syscall::precompiles::k256::K256DecompressEvent;
use crate::syscall::precompiles::keccak256::KeccakPermuteEvent;
use crate::syscall::precompiles::sha256::{ShaCompressEvent, ShaExtendEvent};
use crate::syscall::precompiles::{ECAddEvent, ECDoubleEvent};

/// A record of the execution of a program. Contains event data for everything that happened during
/// the execution of the shard.
#[derive(Default, Clone, Debug)]
pub struct ExecutionRecord {
    /// The index of the shard.
    pub index: u32,

    /// The program.
    pub program: Arc<Program>,

    /// A trace of the CPU events which get emitted during execution.
    pub cpu_events: Vec<CpuEvent>,

    /// A trace of the ADD, and ADDI events.
    pub add_events: Vec<AluEvent>,

    /// A trace of the MUL events.
    pub mul_events: Vec<AluEvent>,

    /// A trace of the SUB events.
    pub sub_events: Vec<AluEvent>,

    /// A trace of the XOR, XORI, OR, ORI, AND, and ANDI events.
    pub bitwise_events: Vec<AluEvent>,

    /// A trace of the SLL and SLLI events.
    pub shift_left_events: Vec<AluEvent>,

    /// A trace of the SRL, SRLI, SRA, and SRAI events.
    pub shift_right_events: Vec<AluEvent>,

    /// A trace of the DIV, DIVU, REM, and REMU events.
    pub divrem_events: Vec<AluEvent>,

    /// A trace of the SLT, SLTI, SLTU, and SLTIU events.
    pub lt_events: Vec<AluEvent>,

    /// A trace of the byte lookups needed.
    pub byte_lookups: BTreeMap<ByteLookupEvent, usize>,

    /// A trace of field LTU events.
    pub field_events: Vec<FieldEvent>,

    pub sha_extend_events: Vec<ShaExtendEvent>,

    pub sha_compress_events: Vec<ShaCompressEvent>,

    pub keccak_permute_events: Vec<KeccakPermuteEvent>,

    pub ed_add_events: Vec<ECAddEvent>,

    pub ed_decompress_events: Vec<EdDecompressEvent>,

    pub weierstrass_add_events: Vec<ECAddEvent>,

    pub weierstrass_double_events: Vec<ECDoubleEvent>,

    pub k256_decompress_events: Vec<K256DecompressEvent>,

    pub blake3_compress_inner_events: Vec<Blake3CompressInnerEvent>,

    /// Information needed for global chips. This shouldn't really be here but for legacy reasons,
    /// we keep this information in this struct for now.
    pub first_memory_record: Vec<(u32, MemoryRecord, u32)>,
    pub last_memory_record: Vec<(u32, MemoryRecord, u32)>,
    pub program_memory_record: Vec<(u32, MemoryRecord, u32)>,
}

#[derive(Debug, Clone, Default)]
pub struct ShardStats {
    pub nb_cpu_events: usize,
    pub nb_add_events: usize,
    pub nb_mul_events: usize,
    pub nb_sub_events: usize,
    pub nb_bitwise_events: usize,
    pub nb_shift_left_events: usize,
    pub nb_shift_right_events: usize,
    pub nb_divrem_events: usize,
    pub nb_lt_events: usize,
    pub nb_field_events: usize,
    pub nb_sha_extend_events: usize,
    pub nb_sha_compress_events: usize,
    pub nb_keccak_permute_events: usize,
    pub nb_ed_add_events: usize,
    pub nb_ed_decompress_events: usize,
    pub nb_weierstrass_add_events: usize,
    pub nb_weierstrass_double_events: usize,
    pub nb_k256_decompress_events: usize,
}

impl ExecutionRecord {
    pub fn add_byte_lookup_event(&mut self, blu_event: ByteLookupEvent) {
        self.byte_lookups
            .entry(blu_event)
            .and_modify(|i| *i += 1)
            .or_insert(1);
    }

    pub fn add_byte_lookup_events(&mut self, blu_events: Vec<ByteLookupEvent>) {
        for blu_event in blu_events.iter() {
            self.add_byte_lookup_event(*blu_event);
        }
    }

    /// Adds a `ByteLookupEvent` to verify `a` and `b are indeed bytes to the shard.
    pub fn add_u8_range_check(&mut self, a: u8, b: u8) {
        self.add_byte_lookup_event(ByteLookupEvent {
            opcode: ByteOpcode::U8Range,
            a1: 0,
            a2: 0,
            b: a as u32,
            c: b as u32,
        });
    }

    /// Adds a `ByteLookupEvent` to verify `a` is indeed u16.
    pub fn add_u16_range_check(&mut self, a: u32) {
        self.add_byte_lookup_event(ByteLookupEvent {
            opcode: ByteOpcode::U16Range,
            a1: a,
            a2: 0,
            b: 0,
            c: 0,
        });
    }

    /// Adds `ByteLookupEvent`s to verify that all the bytes in the input slice are indeed bytes.
    pub fn add_u8_range_checks(&mut self, ls: &[u8]) {
        let mut index = 0;
        while index + 1 < ls.len() {
            self.add_u8_range_check(ls[index], ls[index + 1]);
            index += 2;
        }
        if index < ls.len() {
            // If the input slice's length is odd, we need to add a check for the last byte.
            self.add_u8_range_check(ls[index], 0);
        }
    }

    /// Adds `ByteLookupEvent`s to verify that all the bytes in the input slice are indeed bytes.
    pub fn add_u16_range_checks(&mut self, ls: &[u32]) {
        ls.iter().for_each(|x| self.add_u16_range_check(*x));
    }

    /// Adds a `ByteLookupEvent` to compute the bitwise OR of the two input values.
    pub fn lookup_or(&mut self, b: u8, c: u8) {
        self.add_byte_lookup_event(ByteLookupEvent {
            opcode: ByteOpcode::OR,
            a1: (b | c) as u32,
            a2: 0,
            b: b as u32,
            c: c as u32,
        });
    }

    pub fn add_alu_events(&mut self, alu_events: HashMap<Opcode, Vec<AluEvent>>) {
        for opcode in alu_events.keys() {
            match opcode {
                Opcode::ADD => {
                    self.add_events.extend_from_slice(&alu_events[opcode]);
                }
                Opcode::MUL | Opcode::MULH | Opcode::MULHU | Opcode::MULHSU => {
                    self.mul_events.extend_from_slice(&alu_events[opcode]);
                }
                Opcode::SUB => {
                    self.sub_events.extend_from_slice(&alu_events[opcode]);
                }
                Opcode::XOR | Opcode::OR | Opcode::AND => {
                    self.bitwise_events.extend_from_slice(&alu_events[opcode]);
                }
                Opcode::SLL => {
                    self.shift_left_events
                        .extend_from_slice(&alu_events[opcode]);
                }
                Opcode::SRL | Opcode::SRA => {
                    self.shift_right_events
                        .extend_from_slice(&alu_events[opcode]);
                }
                Opcode::SLT | Opcode::SLTU => {
                    self.lt_events.extend_from_slice(&alu_events[opcode]);
                }
                _ => {
                    panic!("Invalid opcode: {:?}", opcode);
                }
            }
        }
    }

    pub fn stats(&self) -> ShardStats {
        ShardStats {
            nb_cpu_events: self.cpu_events.len(),
            nb_add_events: self.add_events.len(),
            nb_mul_events: self.mul_events.len(),
            nb_sub_events: self.sub_events.len(),
            nb_bitwise_events: self.bitwise_events.len(),
            nb_shift_left_events: self.shift_left_events.len(),
            nb_shift_right_events: self.shift_right_events.len(),
            nb_divrem_events: self.divrem_events.len(),
            nb_lt_events: self.lt_events.len(),
            nb_field_events: self.field_events.len(),
            nb_sha_extend_events: self.sha_extend_events.len(),
            nb_sha_compress_events: self.sha_compress_events.len(),
            nb_keccak_permute_events: self.keccak_permute_events.len(),
            nb_ed_add_events: self.ed_add_events.len(),
            nb_ed_decompress_events: self.ed_decompress_events.len(),
            nb_weierstrass_add_events: self.weierstrass_add_events.len(),
            nb_weierstrass_double_events: self.weierstrass_double_events.len(),
            nb_k256_decompress_events: self.k256_decompress_events.len(),
        }
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct CpuRecord {
    pub a: Option<MemoryRecordEnum>,
    pub b: Option<MemoryRecordEnum>,
    pub c: Option<MemoryRecordEnum>,
    pub memory: Option<MemoryRecordEnum>,
}
