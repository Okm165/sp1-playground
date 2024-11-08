use serde::{Deserialize, Serialize};

use crate::events::{memory::MemoryWriteRecord, LookupId, MemoryLocalEvent};

/// Poseidon2 Permutation Event.
///
/// This event is emitted when a Poseidon2 Permutation operation is performed.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Poseidon2PermEvent {
    /// The lookup identifier.
    pub lookup_id: LookupId,
    /// The shard number.
    pub shard: u32,
    /// The clock cycle.
    pub clk: u32,
    /// The pointer to the x value.
    pub input_ptr: u32,
    /// The memory records for the x value.
    pub input_memory_records: Vec<MemoryWriteRecord>,
    /// The local memory access records.
    pub local_mem_access: Vec<MemoryLocalEvent>,
}
