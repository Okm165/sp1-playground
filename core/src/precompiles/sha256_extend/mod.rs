mod air;
mod columns;
mod flags;
mod trace;

pub use columns::*;

pub struct ShaExtendChip;

impl ShaExtendChip {
    pub fn new() -> Self {
        Self {}
    }
}

pub fn sha_extend(w: &mut [u32]) {
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16] + s0 + w[i - 7] + s1;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use p3_challenger::DuplexChallenger;
    use p3_dft::Radix2DitParallel;
    use p3_field::Field;

    use p3_baby_bear::BabyBear;
    use p3_field::extension::BinomialExtensionField;
    use p3_field::AbstractField;
    use p3_fri::{FriBasedPcs, FriConfigImpl, FriLdt};
    use p3_keccak::Keccak256Hash;
    use p3_ldt::QuotientMmcs;
    use p3_matrix::dense::RowMajorMatrix;
    use p3_mds::coset_mds::CosetMds;
    use p3_merkle_tree::FieldMerkleTreeMmcs;
    use p3_poseidon2::{DiffusionMatrixBabybear, Poseidon2};
    use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher32};
    use p3_uni_stark::{prove, verify, StarkConfigImpl};
    use rand::thread_rng;

    use crate::{
        alu::AluEvent,
        cpu::trace::CpuChip,
        lookup::{debug_interactions, InteractionKind},
        runtime::{Instruction, Opcode, Program, Runtime, Segment},
        utils::Chip,
    };
    use p3_commit::ExtensionMmcs;

    use super::ShaExtendChip;

    #[test]
    fn generate_trace() {
        let mut segment = Segment::default();
        segment.add_events = vec![AluEvent::new(0, Opcode::ADD, 14, 8, 6)];
        let chip = ShaExtendChip::new();
        let trace: RowMajorMatrix<BabyBear> = chip.generate_trace(&mut segment);
        println!("{:?}", trace.values)
    }

    #[test]
    fn prove_babybear() {
        type Val = BabyBear;
        type Domain = Val;
        type Challenge = BinomialExtensionField<Val, 4>;
        type PackedChallenge = BinomialExtensionField<<Domain as Field>::Packing, 4>;

        type MyMds = CosetMds<Val, 16>;
        let mds = MyMds::default();

        type Perm = Poseidon2<Val, MyMds, DiffusionMatrixBabybear, 16, 5>;
        let perm = Perm::new_from_rng(8, 22, mds, DiffusionMatrixBabybear, &mut thread_rng());

        type MyHash = SerializingHasher32<Keccak256Hash>;
        let hash = MyHash::new(Keccak256Hash {});

        type MyCompress = CompressionFunctionFromHasher<Val, MyHash, 2, 8>;
        let compress = MyCompress::new(hash);

        type ValMmcs = FieldMerkleTreeMmcs<Val, MyHash, MyCompress, 8>;
        let val_mmcs = ValMmcs::new(hash, compress);

        type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;
        let challenge_mmcs = ChallengeMmcs::new(val_mmcs.clone());

        type Dft = Radix2DitParallel;
        let dft = Dft {};

        type Challenger = DuplexChallenger<Val, Perm, 16>;

        type Quotient = QuotientMmcs<Domain, Challenge, ValMmcs>;
        type MyFriConfig = FriConfigImpl<Val, Challenge, Quotient, ChallengeMmcs, Challenger>;
        let fri_config = MyFriConfig::new(40, challenge_mmcs);
        let ldt = FriLdt { config: fri_config };

        type Pcs = FriBasedPcs<MyFriConfig, ValMmcs, Dft, Challenger>;
        type MyConfig = StarkConfigImpl<Val, Challenge, PackedChallenge, Pcs, Challenger>;

        let pcs = Pcs::new(dft, val_mmcs, ldt);
        let config = StarkConfigImpl::new(pcs);
        let mut challenger = Challenger::new(perm.clone());

        let w_ptr = 100;
        let mut instructions = vec![Instruction::new(Opcode::ADD, 29, 0, 5, false, true)];
        for i in 0..64 {
            instructions.extend(vec![
                Instruction::new(Opcode::ADD, 30, 0, w_ptr + i * 4, false, true),
                Instruction::new(Opcode::SW, 29, 30, 0, false, true),
            ]);
        }
        instructions.extend(vec![
            Instruction::new(Opcode::ADD, 5, 0, 102, false, true),
            Instruction::new(Opcode::ADD, 10, 0, w_ptr, false, true),
            Instruction::new(Opcode::ECALL, 0, 0, 0, false, false),
        ]);
        let program = Program::new(instructions, 0, 0);
        let mut runtime = Runtime::new(program);
        runtime.run();
        // let mut segment = runtime.segment;

        // let cpu_chip = CpuChip::new();
        // let (_, cpu_count) = debug_interactions::<BabyBear, _>(
        //     cpu_chip,
        //     &mut runtime.segment,
        //     InteractionKind::Memory,
        // );
        // let (_, precompile_count) = debug_interactions::<BabyBear, _>(
        //     ShaExtendChip::new(),
        //     &mut runtime.segment,
        //     InteractionKind::Memory,
        // );

        // let mut final_map = BTreeMap::new();

        // for (key, value) in precompile_count.iter() {
        //     *final_map.entry(key.clone()).or_insert(BabyBear::zero()) += *value;
        // }

        // println!("Final counts");

        // for (key, value) in final_map {
        //     if !value.is_zero() {
        //         println!("Key {} Value {}", key, value);
        //     }
        // }

        runtime
            .segment
            .prove::<_, _, MyConfig>(&config, &mut challenger);
        // let chip = ShaExtendChip::new();
        // let trace: RowMajorMatrix<BabyBear> = chip.generate_trace(&mut segment);
        // let proof = prove::<MyConfig, _>(&config, &chip, &mut challenger, trace);

        // let mut challenger = Challenger::new(perm);
        // verify(&config, &chip, &mut challenger, &proof).unwrap();
    }
}
