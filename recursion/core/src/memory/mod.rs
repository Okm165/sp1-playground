mod air;
mod columns;

use crate::air::Block;
use p3_field::PrimeField32;
use sp1_derive::AlignedBorrow;

#[derive(Debug, Clone)]
pub struct MemoryRecord<F> {
    pub addr: F,
    pub value: Block<F>,
    pub timestamp: F,
    pub prev_value: Block<F>,
    pub prev_timestamp: F,
    _private: (),
}

impl<F: PrimeField32> MemoryRecord<F> {
    pub fn new(
        addr: F,
        value: Block<F>,
        timestamp: F,
        prev_value: Block<F>,
        prev_timestamp: F,
    ) -> Self {
        assert!(timestamp > prev_timestamp);
        Self {
            addr,
            value,
            timestamp,
            prev_value,
            prev_timestamp,
            _private: (),
        }
    }
}

#[derive(AlignedBorrow, Default, Debug, Clone)]
#[repr(C)]
pub struct MemoryReadWriteCols<T> {
    pub addr: T,
    pub value: Block<T>,
    pub timestamp: T,
    pub prev_value: Block<T>,
    pub prev_timestamp: T,
    pub is_real: T,
}

impl<T: Clone> MemoryReadWriteCols<T> {
    pub fn populate(&mut self, record: &MemoryRecord<T>) {
        self.addr = record.addr.clone();
        self.value = record.value.clone();
        self.timestamp = record.timestamp.clone();
        self.prev_value = record.prev_value.clone();
        self.prev_timestamp = record.prev_timestamp.clone();
    }
}

#[allow(dead_code)]
#[derive(PartialEq)]
pub enum MemoryChipKind {
    Init,
    Finalize,
}

pub struct MemoryGlobalChip {
    pub kind: MemoryChipKind,
}
