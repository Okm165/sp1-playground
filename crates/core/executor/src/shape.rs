// use hashbrown::HashMap;
// use p3_baby_bear::BabyBear;
// use p3_field::PrimeField32;
// use sp1_curves::weierstrass::{bls12_381::Bls12381BaseField, bn254::Bn254BaseField};
// use sp1_stark::{air::MachineAir, Chip, Shape};

// use super::riscv_chips::*;
// use crate::{
//     memory::{MemoryChipType, MemoryProgramChip},
//     riscv::RiscvAir,
// };

// lazy_static::lazy_static! {
//     pub static ref SP1_CORE_PROOF_SHAPES: Vec<Shape> = core_proof_shapes::<BabyBear>();
// }

// fn core_proof_shapes<F: PrimeField32>() -> Vec<Shape> {
//     // The order of the chips is used to determine the order of trace generation.
//     let mut chips: Vec<Chip<F, RiscvAir<F>>> = vec![];
//     let cpu = Chip::new(RiscvAir::<F>::Cpu(CpuChip::default()));
//     let program = Chip::new(RiscvAir::<F>::Program(ProgramChip::default()));
//     let sha_extend = Chip::new(RiscvAir::<F>::Sha256Extend(ShaExtendChip::default()));
//     let sha_compress = Chip::new(RiscvAir::<F>::Sha256Compress(ShaCompressChip::default()));
//     let ed_add_assign = Chip::new(RiscvAir::<F>::Ed25519Add(EdAddAssignChip::<
//         EdwardsCurve<Ed25519Parameters>,
//     >::new()));
//     let ed_decompress = Chip::new(RiscvAir::<F>::Ed25519Decompress(EdDecompressChip::<
//         Ed25519Parameters,
//     >::default()));
//     let k256_decompress = Chip::new(RiscvAir::<F>::K256Decompress(WeierstrassDecompressChip::<
//         SwCurve<Secp256k1Parameters>,
//     >::with_lsb_rule()));
//     let secp256k1_add_assign = Chip::new(RiscvAir::<F>::Secp256k1Add(WeierstrassAddAssignChip::<
//         SwCurve<Secp256k1Parameters>,
//     >::new()));
//     let secp256k1_double_assign =
//         Chip::new(RiscvAir::<F>::Secp256k1Double(WeierstrassDoubleAssignChip::<
//             SwCurve<Secp256k1Parameters>,
//         >::new()));
//     let keccak_permute = Chip::new(RiscvAir::<F>::KeccakP(KeccakPermuteChip::new()));
//     let bn254_add_assign = Chip::new(RiscvAir::<F>::Bn254Add(WeierstrassAddAssignChip::<
//         SwCurve<Bn254Parameters>,
//     >::new()));
//     let bn254_double_assign = Chip::new(RiscvAir::<F>::Bn254Double(WeierstrassDoubleAssignChip::<
//         SwCurve<Bn254Parameters>,
//     >::new()));
//     let bls12381_add = Chip::new(RiscvAir::<F>::Bls12381Add(WeierstrassAddAssignChip::<
//         SwCurve<Bls12381Parameters>,
//     >::new()));
//     let bls12381_double = Chip::new(RiscvAir::<F>::Bls12381Double(WeierstrassDoubleAssignChip::<
//         SwCurve<Bls12381Parameters>,
//     >::new()));
//     let uint256_mul = Chip::new(RiscvAir::<F>::Uint256Mul(Uint256MulChip::default()));
//     let bls12381_fp = Chip::new(RiscvAir::<F>::Bls12381Fp(FpOpChip::<Bls12381BaseField>::new()));
//     let bls12381_fp2_addsub = Chip::new(RiscvAir::<F>::Bls12381Fp2AddSub(Fp2AddSubAssignChip::<
//         Bls12381BaseField,
//     >::new()));
//     let bls12381_fp2_mul =
//         Chip::new(RiscvAir::<F>::Bls12381Fp2Mul(Fp2MulAssignChip::<Bls12381BaseField>::new()));
//     let bn254_fp = Chip::new(RiscvAir::<F>::Bn254Fp(FpOpChip::<Bn254BaseField>::new()));
//     let bn254_fp2_addsub =
//         Chip::new(RiscvAir::<F>::Bn254Fp2AddSub(Fp2AddSubAssignChip::<Bn254BaseField>::new()));
//     let bn254_fp2_mul =
//         Chip::new(RiscvAir::<F>::Bn254Fp2Mul(Fp2MulAssignChip::<Bn254BaseField>::new()));
//     let bls12381_decompress =
//         Chip::new(RiscvAir::<F>::Bls12381Decompress(WeierstrassDecompressChip::<
//             SwCurve<Bls12381Parameters>,
//         >::with_lexicographic_rule()));
//     let div_rem = Chip::new(RiscvAir::<F>::DivRem(DivRemChip::default()));
//     let add_sub = Chip::new(RiscvAir::<F>::Add(AddSubChip::default()));
//     let bitwise = Chip::new(RiscvAir::<F>::Bitwise(BitwiseChip::default()));
//     let mul = Chip::new(RiscvAir::<F>::Mul(MulChip::default()));
//     let shift_right = Chip::new(RiscvAir::<F>::ShiftRight(ShiftRightChip::default()));
//     let shift_left = Chip::new(RiscvAir::<F>::ShiftLeft(ShiftLeft::default()));
//     let lt = Chip::new(RiscvAir::<F>::Lt(LtChip::default()));
//     let memory_init =
//         Chip::new(RiscvAir::<F>::MemoryInit(MemoryChip::new(MemoryChipType::Initialize)));
//     let memory_finalize =
//         Chip::new(RiscvAir::<F>::MemoryFinal(MemoryChip::new(MemoryChipType::Finalize)));
//     let memory_program = Chip::new(RiscvAir::<F>::ProgramMemory(MemoryProgramChip::default()));
//     let byte = Chip::new(RiscvAir::<F>::ByteLookup(ByteChip::default()));

//     vec![]
// }

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

/// The shape of a core proof.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoreShape {
    /// The id of the shape. Used for enumeration of the possible proof shapes.
    pub id: usize,
    /// The shape of the proof. Keys are the chip names and values are the log-heights of the chips.
    pub shape: HashMap<String, usize>,
}
