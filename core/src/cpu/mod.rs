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
    pub a_record: Option<MemoryRecordEnum>,
    pub b: u32,
    pub b_record: Option<MemoryRecordEnum>,
    pub c: u32,
    pub c_record: Option<MemoryRecordEnum>,
    pub memory: Option<u32>,
    pub memory_record: Option<MemoryRecordEnum>,
}

#[derive(Debug, Copy, Clone)]
pub enum MemoryRecordEnum {
    Read(MemoryReadRecord),
    Write(MemoryWriteRecord),
}

impl MemoryRecordEnum {
    pub fn value(&self) -> u32 {
        match self {
            MemoryRecordEnum::Read(record) => record.value,
            MemoryRecordEnum::Write(record) => record.value,
        }
    }

    pub fn to_write_record(&self, value: u32) -> MemoryWriteRecord {
        match self {
            MemoryRecordEnum::Read(record) => record.to_write_record(value),
            MemoryRecordEnum::Write(_) => {
                panic!("Cannot convert write record to write record")
            }
        }
    }
}

impl From<MemoryReadRecord> for MemoryRecordEnum {
    fn from(read_record: MemoryReadRecord) -> Self {
        MemoryRecordEnum::Read(read_record)
    }
}

impl From<MemoryWriteRecord> for MemoryRecordEnum {
    fn from(write_record: MemoryWriteRecord) -> Self {
        MemoryRecordEnum::Write(write_record)
    }
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

impl MemoryReadRecord {
    fn to_write_record(self, value: u32) -> MemoryWriteRecord {
        MemoryWriteRecord {
            value,
            segment: self.segment,
            timestamp: self.timestamp,
            prev_value: self.value,
            prev_segment: self.prev_segment,
            prev_timestamp: self.prev_timestamp,
        }
    }
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
