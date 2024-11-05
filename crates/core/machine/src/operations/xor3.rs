use p3_field::{AbstractField, Field};
use sp1_core_executor::{
    events::{ByteLookupEvent, ByteRecord},
    ByteOpcode,
};
use sp1_derive::AlignedBorrow;
use sp1_primitives::consts::WORD_SIZE;
use sp1_stark::{air::SP1AirBuilder, Word};

/// A set of columns needed to compute the xor of two words.
#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct Xor3Operation<T> {
    /// The result of `x ^ y ^ z`.
    pub value: Word<T>,
}

impl<F: Field> Xor3Operation<F> {
    pub fn populate(
        &mut self,
        record: &mut impl ByteRecord,
        shard: u32,
        x: u32,
        y: u32,
        z: u32,
    ) -> u32 {
        let expected = x ^ y;
        let x_bytes = x.to_le_bytes();
        let y_bytes = y.to_le_bytes();
        let z_bytes = z.to_le_bytes();
        for i in 0..WORD_SIZE {
            let xor2 = x_bytes[i] ^ y_bytes[i];
            let xor3 = xor2 ^ z_bytes[i];
            self.value[i] = F::from_canonical_u8(xor3);

            record.add_byte_lookup_event(ByteLookupEvent {
                shard,
                opcode: ByteOpcode::XOR,
                a1: xor2 as u16,
                a2: 0,
                b: x_bytes[i],
                c: y_bytes[i],
            });

            record.add_byte_lookup_event(ByteLookupEvent {
                shard,
                opcode: ByteOpcode::XOR,
                a1: xor3 as u16,
                a2: 0,
                b: xor2,
                c: z_bytes[i],
            });
        }
        expected
    }

    #[allow(unused_variables)]
    pub fn eval<AB: SP1AirBuilder>(
        builder: &mut AB,
        a: Word<AB::Var>,
        b: Word<AB::Var>,
        c: Word<AB::Var>,
        cols: Xor3Operation<AB::Var>,
        is_real: AB::Var,
    ) where
        AB::Var: Field,
    {
        for i in 0..WORD_SIZE {
            let xor2 = Word::<AB::Var>::from(cols.value.to_u32() ^ c.to_u32());

            builder.send_byte(
                AB::F::from_canonical_u32(ByteOpcode::XOR as u32),
                xor2[i],
                a[i],
                b[i],
                is_real,
            );

            builder.send_byte(
                AB::F::from_canonical_u32(ByteOpcode::XOR as u32),
                cols.value[i],
                xor2[i],
                c[i],
                is_real,
            );
        }
    }
}
