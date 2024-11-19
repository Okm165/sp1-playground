use crate::events::{memory::MemoryWriteRecord, LookupId, MemoryLocalEvent, MemoryReadRecord};
use serde::{Deserialize, Serialize};

/// Poseidon2 Permutation Event.
///
/// This event is emitted when a Poseidon2 Permutation operation is performed.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Poseidon2PermuteEvent {
    /// The lookup identifier.
    pub lookup_id: LookupId,
    /// The shard number.
    pub shard: u32,
    /// The clock cycle.
    pub clk: u32,
    /// State
    pub state_values: Vec<u32>,
    /// The pointer to the memory.
    pub memory_ptr: u32,
    /// The memory records for the pre-state.
    pub state_read_records: Vec<MemoryReadRecord>,
    /// The memory records for the post-state.
    pub state_write_records: Vec<MemoryWriteRecord>,
    /// The local memory access records.
    pub local_mem_access: Vec<MemoryLocalEvent>,
}
