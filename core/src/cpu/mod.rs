use crate::runtime::Instruction;

pub mod airs;
pub mod cols;
pub mod trace;

#[derive(Debug, Copy, Clone)]
pub struct CpuEvent {
    pub segment: u32,
    pub clk: u32,
    pub pc: u32,
    pub instruction: Instruction,
    pub a: u32,
    pub a_record: Option<MemoryRecord>,
    pub b: u32,
    pub b_record: Option<MemoryRecord>,
    pub c: u32,
    pub c_record: Option<MemoryRecord>,
    pub memory: Option<u32>,
    pub memory_record: Option<MemoryRecord>,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct MemoryRecord {
    pub value: u32,
    pub segment: u32,
    pub timestamp: u32,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct MemoryReadRecord {
    pub value: u32,
    pub segment: u32,
    pub timestamp: u32,
    pub prev_segment: u32,
    pub prev_timestamp: u32,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct MemoryWriteRecord {
    pub value: u32,
    pub segment: u32,
    pub timestamp: u32,
    pub prev_value: u32,
    pub prev_segment: u32,
    pub prev_timestamp: u32,
}

pub struct CpuChip;

impl CpuChip {
    pub fn new() -> Self {
        Self {}
    }
}
