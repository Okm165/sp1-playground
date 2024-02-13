use core::borrow::Borrow;
use core::borrow::BorrowMut;
use curta_derive::AlignedBorrow;
use p3_field::AbstractField;
use p3_field::Field;
use std::mem::size_of;

use crate::air::CurtaAirBuilder;
use crate::air::Word;
use crate::bytes::ByteLookupEvent;
use crate::bytes::ByteOpcode;
use crate::disassembler::WORD_SIZE;
use crate::runtime::Host;

/// A set of columns needed to compute the and of two words.
#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct AndOperation<T> {
    /// The result of `x & y`.
    pub value: Word<T>,
}

impl<F: Field> AndOperation<F> {
    pub fn populate<H: Host>(&mut self, host: &mut H, x: u32, y: u32) -> u32 {
        let expected = x & y;
        let x_bytes = x.to_le_bytes();
        let y_bytes = y.to_le_bytes();
        for i in 0..WORD_SIZE {
            let and = x_bytes[i] & y_bytes[i];
            self.value[i] = F::from_canonical_u8(and);

            let byte_event = ByteLookupEvent {
                opcode: ByteOpcode::AND,
                a1: and as u32,
                a2: 0,
                b: x_bytes[i] as u32,
                c: y_bytes[i] as u32,
            };
            host.add_byte_lookup_event(byte_event);
        }
        expected
    }

    #[allow(unused_variables)]
    pub fn eval<AB: CurtaAirBuilder>(
        builder: &mut AB,
        a: Word<AB::Var>,
        b: Word<AB::Var>,
        cols: AndOperation<AB::Var>,
        is_real: AB::Var,
    ) {
        for i in 0..WORD_SIZE {
            builder.send_byte(
                AB::F::from_canonical_u32(ByteOpcode::AND as u32),
                cols.value[i],
                a[i],
                b[i],
                is_real,
            );
        }
    }
}
