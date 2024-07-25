use p3_field::{AbstractExtensionField, AbstractField};
use serde::{Deserialize, Serialize};

use crate::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction<F> {
    BaseAlu(BaseAluInstr<F>),
    ExtAlu(ExtAluInstr<F>),
    Mem(MemInstr<F>),
    Poseidon2Skinny(Poseidon2SkinnyInstr<F>),
    Poseidon2Wide(Poseidon2WideInstr<F>),
    ExpReverseBitsLen(ExpReverseBitsInstr<F>),
    HintBits(HintBitsInstr<F>),
    FriFold(FriFoldInstr<F>),
    Print(PrintInstr<F>),
    HintExt2Felts(HintExt2FeltsInstr<F>),
    CommitPV(CommitPVInstr<F>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HintBitsInstr<F> {
    /// Addresses and mults of the output bits.
    pub output_addrs_mults: Vec<(Address<F>, F)>,
    /// Input value to decompose.
    pub input_addr: Address<F>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrintInstr<F> {
    pub field_elt_type: FieldEltType,
    pub addr: Address<F>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HintExt2FeltsInstr<F> {
    /// Addresses and mults of the output bits.
    pub output_addrs_mults: [(Address<F>, F); D],
    /// Input value to decompose.
    pub input_addr: Address<F>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitPVInstr<F> {
    pub pv_hash: [F; DIGEST_SIZE],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FieldEltType {
    Base,
    Extension,
}

pub fn base_alu<F: AbstractField>(
    opcode: BaseAluOpcode,
    mult: u32,
    out: u32,
    in1: u32,
    in2: u32,
) -> Instruction<F> {
    Instruction::BaseAlu(BaseAluInstr {
        opcode,
        mult: F::from_canonical_u32(mult),
        addrs: BaseAluIo {
            out: Address(F::from_canonical_u32(out)),
            in1: Address(F::from_canonical_u32(in1)),
            in2: Address(F::from_canonical_u32(in2)),
        },
    })
}

pub fn ext_alu<F: AbstractField>(
    opcode: ExtAluOpcode,
    mult: u32,
    out: u32,
    in1: u32,
    in2: u32,
) -> Instruction<F> {
    Instruction::ExtAlu(ExtAluInstr {
        opcode,
        mult: F::from_canonical_u32(mult),
        addrs: ExtAluIo {
            out: Address(F::from_canonical_u32(out)),
            in1: Address(F::from_canonical_u32(in1)),
            in2: Address(F::from_canonical_u32(in2)),
        },
    })
}

pub fn mem<F: AbstractField>(
    kind: MemAccessKind,
    mult: u32,
    addr: u32,
    val: u32,
) -> Instruction<F> {
    mem_single(kind, mult, addr, F::from_canonical_u32(val))
}

pub fn mem_single<F: AbstractField>(
    kind: MemAccessKind,
    mult: u32,
    addr: u32,
    val: F,
) -> Instruction<F> {
    mem_block(kind, mult, addr, Block::from(val))
}

pub fn mem_ext<F: AbstractField + Copy, EF: AbstractExtensionField<F>>(
    kind: MemAccessKind,
    mult: u32,
    addr: u32,
    val: EF,
) -> Instruction<F> {
    mem_block(kind, mult, addr, val.as_base_slice().into())
}

pub fn mem_block<F: AbstractField>(
    kind: MemAccessKind,
    mult: u32,
    addr: u32,
    val: Block<F>,
) -> Instruction<F> {
    Instruction::Mem(MemInstr {
        addrs: MemIo {
            inner: Address(F::from_canonical_u32(addr)),
        },
        vals: MemIo { inner: val },
        mult: F::from_canonical_u32(mult),
        kind,
    })
}

pub fn poseidon2_skinny<F: AbstractField>(
    mults: [u32; WIDTH],
    output: [u32; WIDTH],
    input: [u32; WIDTH],
) -> Instruction<F> {
    Instruction::Poseidon2Skinny(Poseidon2SkinnyInstr {
        mults: mults.map(F::from_canonical_u32),
        addrs: Poseidon2Io {
            output: output.map(F::from_canonical_u32).map(Address),
            input: input.map(F::from_canonical_u32).map(Address),
        },
    })
}

pub fn poseidon2_wide<F: AbstractField>(
    mults: [u32; WIDTH],
    output: [u32; WIDTH],
    input: [u32; WIDTH],
) -> Instruction<F> {
    Instruction::Poseidon2Wide(Poseidon2WideInstr {
        mults: mults.map(F::from_canonical_u32),
        addrs: Poseidon2Io {
            output: output.map(F::from_canonical_u32).map(Address),
            input: input.map(F::from_canonical_u32).map(Address),
        },
    })
}

pub fn exp_reverse_bits_len<F: AbstractField>(
    mult: u32,
    base: F,
    exp: Vec<F>,
    result: F,
) -> Instruction<F> {
    Instruction::ExpReverseBitsLen(ExpReverseBitsInstr {
        mult: F::from_canonical_u32(mult),
        addrs: ExpReverseBitsIo {
            base: Address(base),
            exp: exp.into_iter().map(Address).collect(),
            result: Address(result),
        },
    })
}

#[allow(clippy::too_many_arguments)]
pub fn fri_fold<F: AbstractField>(
    z: u32,
    alpha: u32,
    x: u32,
    mat_opening: Vec<u32>,
    ps_at_z: Vec<u32>,
    alpha_pow_input: Vec<u32>,
    ro_input: Vec<u32>,
    alpha_pow_output: Vec<u32>,
    ro_output: Vec<u32>,
    alpha_mults: Vec<u32>,
    ro_mults: Vec<u32>,
) -> Instruction<F> {
    Instruction::FriFold(FriFoldInstr {
        base_single_addrs: FriFoldBaseIo {
            x: Address(F::from_canonical_u32(x)),
        },
        ext_single_addrs: FriFoldExtSingleIo {
            z: Address(F::from_canonical_u32(z)),
            alpha: Address(F::from_canonical_u32(alpha)),
        },
        ext_vec_addrs: FriFoldExtVecIo {
            mat_opening: mat_opening
                .iter()
                .map(|elm| Address(F::from_canonical_u32(*elm)))
                .collect(),
            ps_at_z: ps_at_z
                .iter()
                .map(|elm| Address(F::from_canonical_u32(*elm)))
                .collect(),
            alpha_pow_input: alpha_pow_input
                .iter()
                .map(|elm| Address(F::from_canonical_u32(*elm)))
                .collect(),
            ro_input: ro_input
                .iter()
                .map(|elm| Address(F::from_canonical_u32(*elm)))
                .collect(),
            alpha_pow_output: alpha_pow_output
                .iter()
                .map(|elm| Address(F::from_canonical_u32(*elm)))
                .collect(),
            ro_output: ro_output
                .iter()
                .map(|elm| Address(F::from_canonical_u32(*elm)))
                .collect(),
        },
        alpha_pow_mults: alpha_mults
            .iter()
            .map(|mult| F::from_canonical_u32(*mult))
            .collect(),
        ro_mults: ro_mults
            .iter()
            .map(|mult| F::from_canonical_u32(*mult))
            .collect(),
    })
}
