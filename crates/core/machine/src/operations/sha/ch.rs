use p3_field::{AbstractField, Field};
use sp1_core_executor::{
    events::{Byte3LookupEvent, Byte3Record},
    Byte3Opcode,
};
use sp1_derive::AlignedBorrow;
use sp1_primitives::consts::WORD_SIZE;
use sp1_stark::{air::SP1AirBuilder, Word};

/// A set of columns needed to compute the xor of two words.
#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct CHOperation<T> {
    /// The result of `(x and y) xor ((not x) and z)`.
    pub value: Word<T>,
}

impl<F: Field> CHOperation<F> {
    pub fn populate(
        &mut self,
        record: &mut impl Byte3Record,
        shard: u32,
        x: u32,
        y: u32,
        z: u32,
    ) -> u32 {
        let expected = x ^ y ^ z;
        let x_bytes = x.to_le_bytes();
        let y_bytes = y.to_le_bytes();
        let z_bytes = y.to_le_bytes();
        for i in 0..WORD_SIZE {
            let xor3 = x_bytes[i] ^ y_bytes[i] ^ z_bytes[i];
            self.value[i] = F::from_canonical_u8(xor3);

            let byte3_event = Byte3LookupEvent {
                shard,
                opcode: Byte3Opcode::CH,
                a: x_bytes[i],
                b: y_bytes[i],
                c: z_bytes[i],
                d: xor3,
            };
            record.add_byte3_lookup_event(byte3_event);
        }
        expected
    }

    #[allow(unused_variables)]
    pub fn eval<AB: SP1AirBuilder>(
        builder: &mut AB,
        a: Word<AB::Var>,
        b: Word<AB::Var>,
        c: Word<AB::Var>,
        cols: CHOperation<AB::Var>,
        is_real: AB::Var,
    ) {
        for i in 0..WORD_SIZE {
            builder.send_byte_triple(
                AB::F::from_canonical_u32(Byte3Opcode::CH as u32),
                a[i],
                b[i],
                c[i],
                cols.value[i],
                is_real,
            );
        }
    }
}
