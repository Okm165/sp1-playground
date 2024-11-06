use hashbrown::HashMap;
use p3_field::Field;
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
    fn add_byte3_lookup_event(&mut self, blu_event: Byte3LookupEvent);

    /// Adds a list of sharded [`Byte3LookupEvent`]s to the record.
    fn add_sharded_byte3_lookup_events(
        &mut self,
        sharded_blu_events_vec: Vec<&HashMap<u32, HashMap<Byte3LookupEvent, usize>>>,
    );
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
