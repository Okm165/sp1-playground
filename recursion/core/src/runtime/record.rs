use std::array;
use std::collections::BTreeMap;
use std::sync::Arc;

use p3_field::{AbstractField, PrimeField32};
use sp1_core::stark::{MachineRecord, PROOF_MAX_NUM_PVS};
use std::collections::HashMap;

use super::RecursionProgram;
use crate::air::Block;
use crate::cpu::CpuEvent;
use crate::fri_fold::FriFoldEvent;
use crate::poseidon2::{Poseidon2CompressEvent, Poseidon2HashEvent};
use crate::range_check::RangeCheckEvent;

#[derive(Default, Debug, Clone)]
pub struct ExecutionRecord<F: Default> {
    pub program: Arc<RecursionProgram<F>>,
    pub cpu_events: Vec<CpuEvent<F>>,
    pub poseidon2_compress_events: Vec<Poseidon2CompressEvent<F>>,
    pub poseidon2_hash_events: Vec<Poseidon2HashEvent<F>>,
    pub fri_fold_events: Vec<FriFoldEvent<F>>,
    pub range_check_events: BTreeMap<RangeCheckEvent, usize>,

    // (address, value)
    pub first_memory_record: Vec<(F, Block<F>)>,

    // (address, last_timestamp, last_value)
    pub last_memory_record: Vec<(F, F, Block<F>)>,

    /// The public values.
    pub public_values: Vec<F>,
}

impl<F: Default> ExecutionRecord<F> {
    pub fn add_range_check_events(&mut self, events: &[RangeCheckEvent]) {
        for event in events {
            *self.range_check_events.entry(*event).or_insert(0) += 1;
        }
    }
}

impl<F: PrimeField32> MachineRecord for ExecutionRecord<F> {
    type Config = ();

    fn index(&self) -> u32 {
        0
    }

    fn set_index(&mut self, _: u32) {}

    fn stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        stats.insert("cpu_events".to_string(), self.cpu_events.len());
        stats.insert(
            "poseidon2_events".to_string(),
            self.poseidon2_compress_events.len(),
        );
        stats.insert("fri_fold_events".to_string(), self.fri_fold_events.len());
        stats.insert(
            "range_check_events".to_string(),
            self.range_check_events.len(),
        );
        stats
    }

    // NOTE: This should be unused.
    fn append(&mut self, other: &mut Self) {
        self.cpu_events.append(&mut other.cpu_events);
        self.first_memory_record
            .append(&mut other.first_memory_record);
        self.last_memory_record
            .append(&mut other.last_memory_record);

        // Merge the range check lookups.
        for (range_check_event, count) in std::mem::take(&mut other.range_check_events).into_iter()
        {
            *self
                .range_check_events
                .entry(range_check_event)
                .or_insert(0) += count;
        }
    }

    fn shard(self, _: &Self::Config) -> Vec<Self> {
        vec![self]
    }

    fn public_values<T: AbstractField>(&self) -> Vec<T> {
        let ret: [T; PROOF_MAX_NUM_PVS] = array::from_fn(|i| {
            if i < self.public_values.len() {
                T::from_canonical_u32(self.public_values[i].as_canonical_u32())
            } else {
                T::zero()
            }
        });

        ret.to_vec()
    }
}
