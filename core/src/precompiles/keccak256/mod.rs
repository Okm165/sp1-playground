use std::ops::Range;

use crate::precompiles::{MemoryReadRecord, MemoryWriteRecord};

use p3_keccak_air::{KeccakAir, NUM_KECCAK_COLS as P3_NUM_KECCAK_COLS};

use self::columns::P3_KECCAK_COLS_OFFSET;

mod air;
pub mod columns;
mod execute;
mod trace;

const NUM_ROUNDS: usize = 24;
const STATE_SIZE: usize = 25;

// The permutation state is 25 u64's.  Our word size is 32 bits, so it is 50 words.
const STATE_NUM_WORDS: usize = 25 * 2;

#[derive(Debug, Clone, Copy)]
pub struct KeccakPermuteEvent {
    pub clk: u32,
    pub pre_state: [u64; STATE_SIZE],
    pub post_state: [u64; STATE_SIZE],
    pub state_read_records: [MemoryReadRecord; STATE_NUM_WORDS],
    pub state_write_records: [MemoryWriteRecord; STATE_NUM_WORDS],
    pub state_addr: u32,
}

pub struct KeccakPermuteChip {
    p3_keccak: KeccakAir,
    p3_keccak_col_range: Range<usize>,
}

impl KeccakPermuteChip {
    pub fn new() -> Self {
        // Get offset of p3_keccak_cols in KeccakCols
        let p3_keccak_air = KeccakAir {};
        Self {
            p3_keccak: p3_keccak_air,
            p3_keccak_col_range: P3_KECCAK_COLS_OFFSET
                ..(P3_KECCAK_COLS_OFFSET + P3_NUM_KECCAK_COLS),
        }
    }
}

#[cfg(test)]
pub mod permute_tests {
    use log::debug;
    use p3_challenger::DuplexChallenger;
    use p3_dft::Radix2DitParallel;
    use p3_field::Field;

    use p3_baby_bear::BabyBear;
    use p3_field::extension::BinomialExtensionField;
    use p3_fri::{FriBasedPcs, FriConfigImpl, FriLdt};
    use p3_keccak::Keccak256Hash;
    use p3_ldt::QuotientMmcs;
    use p3_mds::coset_mds::CosetMds;
    use p3_merkle_tree::FieldMerkleTreeMmcs;
    use p3_poseidon2::{DiffusionMatrixBabybear, Poseidon2};
    use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher32};
    use p3_uni_stark::StarkConfigImpl;
    use rand::thread_rng;

    use crate::{
        runtime::{Instruction, Opcode, Program, Runtime},
        utils::prove,
    };
    use p3_commit::ExtensionMmcs;

    pub fn keccak_permute_program() -> Program {
        let digest_ptr = 100;
        let mut instructions = vec![Instruction::new(Opcode::ADD, 29, 0, 1, false, true)];
        for i in 0..(25 * 8) {
            instructions.extend(vec![
                Instruction::new(Opcode::ADD, 30, 0, digest_ptr + i * 4, false, true),
                Instruction::new(Opcode::SW, 29, 30, 0, false, true),
            ]);
        }
        instructions.extend(vec![
            Instruction::new(Opcode::ADD, 5, 0, 105, false, true),
            Instruction::new(Opcode::ADD, 10, 0, digest_ptr, false, true),
            Instruction::new(Opcode::ECALL, 10, 5, 0, false, true),
        ]);

        Program::new(instructions, 0, 0)
    }

    #[test]
    pub fn test_keccak_permute_program_execute() {
        let program = keccak_permute_program();
        let mut runtime = Runtime::new(program);
        runtime.write_witness(&[999]);
        runtime.run()
    }

    #[test]
    fn prove_babybear() {
        if env_logger::try_init().is_err() {
            debug!("Logger already initialized")
        }

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
        let fri_config = MyFriConfig::new(1, 40, 8, challenge_mmcs);
        let ldt = FriLdt { config: fri_config };

        type Pcs = FriBasedPcs<MyFriConfig, ValMmcs, Dft, Challenger>;
        type MyConfig = StarkConfigImpl<Val, Challenge, PackedChallenge, Pcs, Challenger>;

        let pcs = Pcs::new(dft, val_mmcs, ldt);
        let config = StarkConfigImpl::new(pcs);
        let mut challenger = Challenger::new(perm.clone());

        let program = keccak_permute_program();
        let mut runtime = tracing::info_span!("runtime.run(...)").in_scope(|| {
            let mut runtime = Runtime::new(program);
            runtime.write_witness(&[999]);
            runtime.run();
            runtime
        });

        tracing::info_span!("runtime.prove(...)").in_scope(|| {
            runtime.prove::<_, _, MyConfig>(&config, &mut challenger);
        });
    }

    #[test]
    fn test_keccak_permute_program_prove() {
        let program = Program::from_elf("../programs/keccak_permute");
        prove(program);
    }
}
