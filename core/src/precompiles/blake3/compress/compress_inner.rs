use core::borrow::Borrow;
use core::borrow::BorrowMut;
use num::integer::Roots;
use p3_air::AirBuilder;
use p3_field::AbstractField;
use p3_field::Field;

use std::mem::size_of;
use valida_derive::AlignedBorrow;

use crate::air::CurtaAirBuilder;

use super::round::RoundOperation;
/// A set of columns needed to compute the `add_rc` of the input state.
#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct CompressInnnerOperation<T> {
    pub result: [T; 16],
    pub rounds: [RoundOperation<T>; 7],
}
