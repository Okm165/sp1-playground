use std::iter::once;

use poseidon2_wide::WIDTH;
use serde::{Deserialize, Serialize};
use sp1_derive::AlignedBorrow;
use sp1_recursion_core::air::Block;

pub mod alu_base;
pub mod alu_ext;
pub mod builder;
pub mod exp_reverse_bits;
pub mod fri_fold;
pub mod machine;
pub mod mem;
pub mod poseidon2_wide;
pub mod program;
pub mod runtime;

pub use runtime::*;

#[derive(AlignedBorrow, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct Address<F>(pub F);

// -------------------------------------------------------------------------------------------------

/// The inputs and outputs to an operation of the base field ALU.
#[derive(AlignedBorrow, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseAluIo<V> {
    pub out: V,
    pub in1: V,
    pub in2: V,
}

pub type BaseAluEvent<F> = BaseAluIo<F>;

/// An instruction invoking the extension field ALU.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseAluInstr<F> {
    pub opcode: BaseAluOpcode,
    pub mult: F,
    pub addrs: BaseAluIo<Address<F>>,
}

// -------------------------------------------------------------------------------------------------

/// The inputs and outputs to an operation of the extension field ALU.
#[derive(AlignedBorrow, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtAluIo<V> {
    pub out: V,
    pub in1: V,
    pub in2: V,
}

pub type ExtAluEvent<F> = ExtAluIo<Block<F>>;

/// An instruction invoking the extension field ALU.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtAluInstr<F> {
    pub opcode: ExtAluOpcode,
    pub mult: F,
    pub addrs: ExtAluIo<Address<F>>,
}

// -------------------------------------------------------------------------------------------------

/// The inputs and outputs to the manual memory management/memory initialization table.
#[derive(AlignedBorrow, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemIo<V> {
    pub inner: V,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemInstr<F> {
    pub addrs: MemIo<Address<F>>,
    pub vals: MemIo<Block<F>>,
    pub mult: F,
    pub kind: MemAccessKind,
}

pub type MemEvent<F> = MemIo<Block<F>>;

// -------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MemAccessKind {
    Read,
    Write,
}

/// The inputs and outputs to a Poseidon2 permutation.
#[derive(AlignedBorrow, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Poseidon2Io<V> {
    pub input: [V; WIDTH],
    pub output: [V; WIDTH],
}

/// An instruction invoking the Poseidon2 permutation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Poseidon2WideInstr<F> {
    pub addrs: Poseidon2Io<Address<F>>,
    pub mult: F,
}

pub type Poseidon2WideEvent<F> = Poseidon2Io<F>;

/// The inputs and outputs to an exp-reverse-bits operation.
#[derive(AlignedBorrow, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpReverseBitsIo<V> {
    pub base: V,
    // The bits of the exponent in little-endian order in a vec.
    pub exp: Vec<V>,
    pub result: V,
}

/// An instruction invoking the exp-reverse-bits operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpReverseBitsInstr<F> {
    pub addrs: ExpReverseBitsIo<Address<F>>,
    pub mult: F,
}

/// The event encoding the inputs and outputs of an exp-reverse-bits operation. The `len` operand is
/// now stored as the length of the `exp` field.
#[derive(AlignedBorrow, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpReverseBitsEvent<F> {
    pub base: F,
    pub exp: Vec<F>,
    pub result: F,
}

/// The inputs to a FriFold invocation. There are three types of inputs: base field single inputs,
/// extension field single inputs, and extension field vector inputs.
#[derive(AlignedBorrow, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriFoldIo<V> {
    pub ext_single: FriFoldExtSingleIo<Block<V>>,
    pub ext_vec: FriFoldExtVecIo<Vec<Block<V>>>,
    pub base: FriFoldBaseIo<V>,
}
/// The extension-field-valued single inputs to the FRI fold operation.
#[derive(AlignedBorrow, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriFoldExtSingleIo<V> {
    pub z: V,
    pub alpha: V,
}

/// The extension-field-valued vector inputs to the FRI fold operation.
#[derive(AlignedBorrow, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriFoldExtVecIo<V> {
    pub mat_opening: V,
    pub ps_at_z: V,
    pub alpha_pow: V,
    pub ro: V,
}

/// The base-field-valued inputs to the FRI fold operation.
#[derive(AlignedBorrow, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriFoldBaseIo<V> {
    pub x: V,
}

/// An instruction invoking the FRI fold operation. Addresses for extension field elements are of the
/// same type as for base field elements.
pub struct FriFoldInstr<F> {
    pub single_addrs: FriFoldBaseIo<Address<F>>,
    pub ext_single_addrs: FriFoldExtSingleIo<Address<F>>,
    pub ext_vec_addrs: FriFoldExtVecIo<Vec<Address<F>>>,
}

/// The event encoding the data of a single iteration within the FRI fold operation.
/// For any given event, we are accessing a single element of the `Vec` inputs, so that the event
/// is not a type alias for `FriFoldIo` like many of the other events.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriFoldEvent<F> {
    pub base: FriFoldBaseIo<F>,
    pub ext_single: FriFoldExtSingleIo<Block<F>>,
    pub vec_accesses: FriFoldExtVecIo<Block<F>>,
}
