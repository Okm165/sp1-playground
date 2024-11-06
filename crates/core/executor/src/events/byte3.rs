use hashbrown::HashMap;
use itertools::Itertools;
use p3_field::Field;
use p3_maybe_rayon::prelude::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use serde::{Deserialize, Serialize};
use std::hash::Hash;

use crate::Byte3Opcode;

/// The number of different byte3 operations.
pub const NUM_BYTE3_OPS: usize = 3;

/// Byte3 Lookup Event.
///
/// This object encapsulates the information needed to prove a byte3 lookup operation. This includes
/// the shard, opcode, operands, and other relevant information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct Byte3LookupEvent {
    /// The shard number.
    pub shard: u32,
    /// The opcode.
    pub opcode: Byte3Opcode,
    /// The first operand.
    pub a: u8,
    /// The second operand.
    pub b: u8,
    /// The third operand.
    pub c: u8,
    /// The forth operand.
    pub d: u8,
}

/// A type that can record byte3 lookup events.
pub trait Byte3Record {
    /// Adds a new [`Byte3LookupEvent`] to the record.
    fn add_byte3_lookup_event(&mut self, b3lu_event: Byte3LookupEvent);

    /// Adds a list of sharded [`Byte3LookupEvent`]s to the record.
    fn add_sharded_byte3_lookup_events(
        &mut self,
        sharded_b3lu_events_vec: Vec<&HashMap<u32, HashMap<Byte3LookupEvent, usize>>>,
    );
}

impl Byte3Record for Vec<Byte3LookupEvent> {
    fn add_byte3_lookup_event(&mut self, b3lu_event: Byte3LookupEvent) {
        self.push(b3lu_event);
    }

    fn add_sharded_byte3_lookup_events(
        &mut self,
        _: Vec<&HashMap<u32, HashMap<Byte3LookupEvent, usize>>>,
    ) {
        todo!()
    }
}

impl Byte3Record for HashMap<u32, HashMap<Byte3LookupEvent, usize>> {
    #[inline]
    fn add_byte3_lookup_event(&mut self, b3lu_event: Byte3LookupEvent) {
        self.entry(b3lu_event.shard)
            .or_default()
            .entry(b3lu_event)
            .and_modify(|e| *e += 1)
            .or_insert(1);
    }

    fn add_sharded_byte3_lookup_events(
        &mut self,
        new_events: Vec<&HashMap<u32, HashMap<Byte3LookupEvent, usize>>>,
    ) {
        add_sharded_byte3_lookup_events(self, new_events);
    }
}

pub(crate) fn add_sharded_byte3_lookup_events(
    sharded_b3lu_events: &mut HashMap<u32, HashMap<Byte3LookupEvent, usize>>,
    new_events: Vec<&HashMap<u32, HashMap<Byte3LookupEvent, usize>>>,
) {
    // new_sharded_b3lu_map is a map of shard -> Vec<map of byte lookup event -> multiplicities>.
    // We want to collect the new events in this format so that we can do parallel aggregation
    // per shard.
    let mut new_sharded_b3lu_map: HashMap<u32, Vec<&HashMap<Byte3LookupEvent, usize>>> =
        HashMap::new();
    for new_sharded_b3lu_events in new_events {
        for (shard, new_b3lu_map) in new_sharded_b3lu_events {
            new_sharded_b3lu_map.entry(*shard).or_insert(Vec::new()).push(new_b3lu_map);
        }
    }

    // Collect all the shard numbers.
    let shards: Vec<u32> = new_sharded_b3lu_map.keys().copied().collect_vec();

    // Move ownership of self's per shard b3lu maps into a vec.  This is so that we
    // can do parallel aggregation per shard.
    let mut self_b3lu_maps: Vec<HashMap<Byte3LookupEvent, usize>> = Vec::new();
    for shard in &shards {
        let b3lu = sharded_b3lu_events.remove(shard);

        match b3lu {
            Some(b3lu) => {
                self_b3lu_maps.push(b3lu);
            }
            None => {
                self_b3lu_maps.push(HashMap::new());
            }
        }
    }

    // Increment self's byte lookup events multiplicity.
    shards.par_iter().zip_eq(self_b3lu_maps.par_iter_mut()).for_each(|(shard, self_b3lu_map)| {
        let b3lu_map_vec = new_sharded_b3lu_map.get(shard).unwrap();
        for b3lu_map in b3lu_map_vec.iter() {
            for (b3lu_event, count) in b3lu_map.iter() {
                *self_b3lu_map.entry(*b3lu_event).or_insert(0) += count;
            }
        }
    });

    // Move ownership of the b3lu maps back to self.
    for (shard, b3lu) in shards.into_iter().zip(self_b3lu_maps.into_iter()) {
        sharded_b3lu_events.insert(shard, b3lu);
    }
}

impl Byte3LookupEvent {
    /// Creates a new `Byte3LookupEvent`.
    #[must_use]
    pub fn new(shard: u32, opcode: Byte3Opcode, a: u8, b: u8, c: u8, d: u8) -> Self {
        Self { shard, opcode, a, b, c, d }
    }
}

impl Byte3Opcode {
    /// Get all the byte3 opcodes.
    #[must_use]
    pub fn all() -> Vec<Self> {
        let opcodes = vec![Byte3Opcode::XOR3, Byte3Opcode::CH, Byte3Opcode::MAJ];
        debug_assert_eq!(opcodes.len(), NUM_BYTE3_OPS);
        opcodes
    }

    /// Convert the opcode to a field element.
    #[must_use]
    pub fn as_field<F: Field>(self) -> F {
        F::from_canonical_u8(self as u8)
    }
}
