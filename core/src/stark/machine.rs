use crate::air::MachineAir;
use crate::alu::AddChip;
use crate::alu::BitwiseChip;
use crate::alu::DivRemChip;
use crate::alu::LtChip;
use crate::alu::MulChip;
use crate::alu::ShiftLeft;
use crate::alu::ShiftRightChip;
use crate::alu::SubChip;
use crate::bytes::ByteChip;
use crate::cpu::CpuChip;
use crate::field::FieldLTUChip;
use crate::memory::MemoryChipKind;
use crate::memory::MemoryGlobalChip;
use crate::program::ProgramChip;
use crate::runtime::ExecutionRecord;
use crate::syscall::precompiles::edwards::EdAddAssignChip;
use crate::syscall::precompiles::edwards::EdDecompressChip;
use crate::syscall::precompiles::k256::K256DecompressChip;
use crate::syscall::precompiles::keccak256::KeccakPermuteChip;
use crate::syscall::precompiles::sha256::ShaCompressChip;
use crate::syscall::precompiles::sha256::ShaExtendChip;
use crate::syscall::precompiles::weierstrass::WeierstrassAddAssignChip;
use crate::syscall::precompiles::weierstrass::WeierstrassDoubleAssignChip;
use crate::utils::ec::edwards::ed25519::Ed25519Parameters;
use crate::utils::ec::edwards::EdwardsCurve;
use crate::utils::ec::weierstrass::secp256k1::Secp256k1Parameters;
use crate::utils::ec::weierstrass::SWCurve;
use p3_air::BaseAir;
use p3_challenger::CanObserve;
use p3_commit::Pcs;
use p3_field::AbstractField;
use p3_field::Field;
use p3_field::PrimeField32;
use p3_matrix::dense::RowMajorMatrix;
use p3_maybe_rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::Chip;
use super::ChipRef;
use super::Com;
use super::MainData;
use super::OpeningProof;
use super::PcsProverData;
use super::Proof;
use super::Prover;
use super::StarkGenericConfig;
use super::VerificationError;
use super::Verifier;

pub struct ProverData<SC: StarkGenericConfig> {
    pub preprocessed_traces: Vec<Option<RowMajorMatrix<SC::Val>>>,
    pub preprocessed_data: Option<PcsProverData<SC>>,
}

pub struct PublicParameters<SC: StarkGenericConfig> {
    pub preprocessed_commitment: Option<Com<SC>>,
}

pub struct RiscvStark<SC: StarkGenericConfig> {
    config: SC,

    program: Chip<SC::Val, ProgramChip>,
    cpu: Chip<SC::Val, CpuChip>,
    sha_extend: Chip<SC::Val, ShaExtendChip>,
    sha_compress: Chip<SC::Val, ShaCompressChip>,
    ed_add_assign: Chip<SC::Val, EdAddAssignChip<EdwardsCurve<Ed25519Parameters>>>,
    ed_decompress: Chip<SC::Val, EdDecompressChip<Ed25519Parameters>>,
    k256_decompress: Chip<SC::Val, K256DecompressChip>,
    weierstrass_add_assign: Chip<SC::Val, WeierstrassAddAssignChip<SWCurve<Secp256k1Parameters>>>,
    weierstrass_double_assign:
        Chip<SC::Val, WeierstrassDoubleAssignChip<SWCurve<Secp256k1Parameters>>>,
    keccak_permute: Chip<SC::Val, KeccakPermuteChip>,
    add: Chip<SC::Val, AddChip>,
    sub: Chip<SC::Val, SubChip>,
    bitwise: Chip<SC::Val, BitwiseChip>,
    div_rem: Chip<SC::Val, DivRemChip>,
    mul: Chip<SC::Val, MulChip>,
    shift_right: Chip<SC::Val, ShiftRightChip>,
    shift_left: Chip<SC::Val, ShiftLeft>,
    lt: Chip<SC::Val, LtChip>,
    field_ltu: Chip<SC::Val, FieldLTUChip>,
    byte: Chip<SC::Val, ByteChip>,

    memory_init: Chip<SC::Val, MemoryGlobalChip>,
    memory_finalize: Chip<SC::Val, MemoryGlobalChip>,
    program_memory_init: Chip<SC::Val, MemoryGlobalChip>,

    // Commitment to the preprocessed data
    preprocessed_commitment: Option<Com<SC>>,
}

impl<SC: StarkGenericConfig> RiscvStark<SC>
where
    SC::Val: PrimeField32,
{
    pub fn init(config: SC) -> (Self, ProverData<SC>) {
        let program = Chip::new(ProgramChip::default());
        let cpu = Chip::new(CpuChip::default());
        let sha_extend = Chip::new(ShaExtendChip::default());
        let sha_compress = Chip::new(ShaCompressChip::default());
        let ed_add_assign = Chip::new(EdAddAssignChip::<EdwardsCurve<Ed25519Parameters>>::new());
        let ed_decompress = Chip::new(EdDecompressChip::<Ed25519Parameters>::default());
        let k256_decompress = Chip::new(K256DecompressChip::default());
        let weierstrass_add_assign =
            Chip::new(WeierstrassAddAssignChip::<SWCurve<Secp256k1Parameters>>::new());
        let weierstrass_double_assign =
            Chip::new(WeierstrassDoubleAssignChip::<SWCurve<Secp256k1Parameters>>::new());
        let keccak_permute = Chip::new(KeccakPermuteChip::new());
        let add = Chip::new(AddChip::default());
        let sub = Chip::new(SubChip::default());
        let bitwise = Chip::new(BitwiseChip::default());
        let div_rem = Chip::new(DivRemChip::default());
        let mul = Chip::new(MulChip::default());
        let shift_right = Chip::new(ShiftRightChip::default());
        let shift_left = Chip::new(ShiftLeft::default());
        let lt = Chip::new(LtChip::default());
        let field_ltu = Chip::new(FieldLTUChip::default());
        let byte = Chip::new(ByteChip::default());
        let memory_init = Chip::new(MemoryGlobalChip::new(MemoryChipKind::Init));
        let memory_finalize = Chip::new(MemoryGlobalChip::new(MemoryChipKind::Finalize));
        let program_memory_init = Chip::new(MemoryGlobalChip::new(MemoryChipKind::Program));

        let mut machine = Self {
            config,
            program,
            cpu,
            sha_extend,
            sha_compress,
            ed_add_assign,
            ed_decompress,
            k256_decompress,
            weierstrass_add_assign,
            weierstrass_double_assign,
            keccak_permute,
            add,
            sub,
            bitwise,
            div_rem,
            mul,
            shift_right,
            shift_left,
            lt,
            field_ltu,
            byte,
            memory_init,
            memory_finalize,
            program_memory_init,

            preprocessed_commitment: None,
        };

        // Compute commitments to the preprocessed data
        let preprocessed_traces = machine
            .chips()
            .iter()
            .map(|chip| chip.preprocessed_trace())
            .collect::<Vec<_>>();
        let traces = preprocessed_traces
            .iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>();
        let (commit, data) = if !traces.is_empty() {
            Some(machine.config.pcs().commit_batches(traces))
        } else {
            None
        }
        .unzip();

        // Store the commitments in the machine
        machine.preprocessed_commitment = commit;

        (
            machine,
            ProverData {
                preprocessed_traces,
                preprocessed_data: data,
            },
        )
    }

    pub fn chips(&self) -> [ChipRef<SC>; 23] {
        [
            self.program.as_ref(),
            self.cpu.as_ref(),
            self.sha_extend.as_ref(),
            self.sha_compress.as_ref(),
            self.ed_add_assign.as_ref(),
            self.ed_decompress.as_ref(),
            self.k256_decompress.as_ref(),
            self.weierstrass_add_assign.as_ref(),
            self.weierstrass_double_assign.as_ref(),
            self.keccak_permute.as_ref(),
            self.add.as_ref(),
            self.sub.as_ref(),
            self.bitwise.as_ref(),
            self.div_rem.as_ref(),
            self.mul.as_ref(),
            self.shift_right.as_ref(),
            self.shift_left.as_ref(),
            self.lt.as_ref(),
            self.field_ltu.as_ref(),
            self.byte.as_ref(),
            self.memory_init.as_ref(),
            self.memory_finalize.as_ref(),
            self.program_memory_init.as_ref(),
        ]
    }

    /// Prove the program.
    ///
    /// The function returns a vector of segment proofs, one for each segment, and a global proof.
    pub fn prove<P>(
        &self,
        prover_data: &ProverData<SC>,
        record: &mut ExecutionRecord,
        challenger: &mut SC::Challenger,
    ) -> Proof<SC>
    where
        P: Prover<SC>,
        SC: Send + Sync,
        SC::Challenger: Clone,
        <SC::Pcs as Pcs<SC::Val, RowMajorMatrix<SC::Val>>>::Commitment: Send + Sync,
        <SC::Pcs as Pcs<SC::Val, RowMajorMatrix<SC::Val>>>::ProverData: Send + Sync,
        MainData<SC>: Serialize + DeserializeOwned,
        OpeningProof<SC>: Send + Sync,
    {
        // Get the local and global chips.
        let chips = self.chips();

        // Generate the trace for each chip to collect events emitted from chips with dependencies.
        chips.iter().for_each(|chip| {
            chip.generate_trace(record);
        });

        // Display the statistics about the workload.
        tracing::info!("{:#?}", record.stats());

        // For each chip, shard the events into segments.
        let mut shards: Vec<ExecutionRecord> = Vec::new();
        chips.iter().for_each(|chip| {
            chip.shard(record, &mut shards);
        });

        // Generate and commit the traces for each segment.
        let (shard_commits, shard_data) = P::commit_shards(&self.config, &mut shards, &chips);

        // Observe the challenges for each segment.
        shard_commits.into_iter().for_each(|commitment| {
            challenger.observe(commitment);
        });

        // Generate a proof for each segment. Note that we clone the challenger so we can observe
        // identical global challenges across the segments.
        let shard_proofs = shard_data
            .into_par_iter()
            .map(|data| {
                let data = data.materialize().expect("failed to load shard main data");
                let chips = self
                    .chips()
                    .into_iter()
                    .filter(|chip| data.chip_ids.contains(&chip.name()))
                    .collect::<Vec<_>>();
                P::prove_shard(
                    &self.config,
                    &mut challenger.clone(),
                    &chips,
                    data,
                    &prover_data.preprocessed_traces,
                    &prover_data.preprocessed_data,
                )
            })
            .collect::<Vec<_>>();

        Proof { shard_proofs }
    }

    pub fn verify(
        &self,
        challenger: &mut SC::Challenger,
        proof: &Proof<SC>,
    ) -> Result<(), ProgramVerificationError>
    where
        SC::Val: PrimeField32,
        SC::Challenger: Clone,
    {
        // TODO: Observe the challenges in a tree-like structure for easily verifiable reconstruction
        // in a map-reduce recursion setting.
        #[cfg(feature = "perf")]
        tracing::info_span!("observe challenges for all segments").in_scope(|| {
            proof.shard_proofs.iter().for_each(|proof| {
                challenger.observe(proof.commitment.main_commit.clone());
            });
        });

        // Verify the segment proofs.
        for (i, proof) in proof.shard_proofs.iter().enumerate() {
            tracing::info_span!("verifying segment", segment = i).in_scope(|| {
                let chips = self
                    .chips()
                    .into_iter()
                    .filter(|chip| proof.chip_ids.contains(&chip.name()))
                    .collect::<Vec<_>>();
                Verifier::verify_shard(&self.config, &chips, &mut challenger.clone(), proof)
                    .map_err(ProgramVerificationError::InvalidSegmentProof)
            })?;
        }

        // Verify the cumulative sum is 0.
        let mut sum = SC::Challenge::zero();
        #[cfg(feature = "perf")]
        {
            for proof in proof.shard_proofs.iter() {
                sum += proof.cumulative_sum();
            }
        }

        match sum.is_zero() {
            true => Ok(()),
            false => Err(ProgramVerificationError::NonZeroCumulativeSum),
        }
    }
}

#[derive(Debug)]
pub enum ProgramVerificationError {
    InvalidSegmentProof(VerificationError),
    InvalidGlobalProof(VerificationError),
    NonZeroCumulativeSum,
}

#[cfg(test)]
#[allow(non_snake_case)]
pub mod tests {

    use crate::runtime::tests::ecall_lwa_program;
    use crate::runtime::tests::fibonacci_program;
    use crate::runtime::tests::simple_memory_program;
    use crate::runtime::tests::simple_program;
    use crate::runtime::Instruction;
    use crate::runtime::Opcode;
    use crate::runtime::Program;
    use crate::utils;
    use crate::utils::prove;
    use crate::utils::setup_logger;

    #[test]
    fn test_simple_prove() {
        let program = simple_program();
        prove(program);
    }

    #[test]
    fn test_ecall_lwa_prove() {
        let program = ecall_lwa_program();
        prove(program);
    }

    #[test]
    fn test_shift_prove() {
        let shift_ops = [Opcode::SRL, Opcode::SRA, Opcode::SLL];
        let operands = [
            (1, 1),
            (1234, 5678),
            (0xffff, 0xffff - 1),
            (u32::MAX - 1, u32::MAX),
            (u32::MAX, 0),
        ];
        for shift_op in shift_ops.iter() {
            for op in operands.iter() {
                let instructions = vec![
                    Instruction::new(Opcode::ADD, 29, 0, op.0, false, true),
                    Instruction::new(Opcode::ADD, 30, 0, op.1, false, true),
                    Instruction::new(*shift_op, 31, 29, 3, false, false),
                ];
                let program = Program::new(instructions, 0, 0);
                prove(program);
            }
        }
    }

    #[test]
    fn test_sub_prove() {
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 0, 8, false, true),
            Instruction::new(Opcode::SUB, 31, 30, 29, false, false),
        ];
        let program = Program::new(instructions, 0, 0);
        prove(program);
    }

    #[test]
    fn test_add_prove() {
        setup_logger();
        let instructions = vec![
            Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
            Instruction::new(Opcode::ADD, 30, 0, 8, false, true),
            Instruction::new(Opcode::ADD, 31, 30, 29, false, false),
        ];
        let program = Program::new(instructions, 0, 0);
        prove(program);
    }

    #[test]
    fn test_mul_prove() {
        let mul_ops = [Opcode::MUL, Opcode::MULH, Opcode::MULHU, Opcode::MULHSU];
        utils::setup_logger();
        let operands = [
            (1, 1),
            (1234, 5678),
            (8765, 4321),
            (0xffff, 0xffff - 1),
            (u32::MAX - 1, u32::MAX),
        ];
        for mul_op in mul_ops.iter() {
            for operand in operands.iter() {
                let instructions = vec![
                    Instruction::new(Opcode::ADD, 29, 0, operand.0, false, true),
                    Instruction::new(Opcode::ADD, 30, 0, operand.1, false, true),
                    Instruction::new(*mul_op, 31, 30, 29, false, false),
                ];
                let program = Program::new(instructions, 0, 0);
                prove(program);
            }
        }
    }

    #[test]
    fn test_lt_prove() {
        let less_than = [Opcode::SLT, Opcode::SLTU];
        for lt_op in less_than.iter() {
            let instructions = vec![
                Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
                Instruction::new(Opcode::ADD, 30, 0, 8, false, true),
                Instruction::new(*lt_op, 31, 30, 29, false, false),
            ];
            let program = Program::new(instructions, 0, 0);
            prove(program);
        }
    }

    #[test]
    fn test_bitwise_prove() {
        let bitwise_opcodes = [Opcode::XOR, Opcode::OR, Opcode::AND];

        for bitwise_op in bitwise_opcodes.iter() {
            let instructions = vec![
                Instruction::new(Opcode::ADD, 29, 0, 5, false, true),
                Instruction::new(Opcode::ADD, 30, 0, 8, false, true),
                Instruction::new(*bitwise_op, 31, 30, 29, false, false),
            ];
            let program = Program::new(instructions, 0, 0);
            prove(program);
        }
    }

    #[test]
    fn test_divrem_prove() {
        let div_rem_ops = [Opcode::DIV, Opcode::DIVU, Opcode::REM, Opcode::REMU];
        let operands = [
            (1, 1),
            (123, 456 * 789),
            (123 * 456, 789),
            (0xffff * (0xffff - 1), 0xffff),
            (u32::MAX - 5, u32::MAX - 7),
        ];
        for div_rem_op in div_rem_ops.iter() {
            for op in operands.iter() {
                let instructions = vec![
                    Instruction::new(Opcode::ADD, 29, 0, op.0, false, true),
                    Instruction::new(Opcode::ADD, 30, 0, op.1, false, true),
                    Instruction::new(*div_rem_op, 31, 29, 30, false, false),
                ];
                let program = Program::new(instructions, 0, 0);
                prove(program);
            }
        }
    }

    #[test]
    fn test_fibonacci_prove() {
        setup_logger();
        let program = fibonacci_program();
        prove(program);
    }

    #[test]
    fn test_simple_memory_program_prove() {
        let program = simple_memory_program();
        prove(program);
    }
}
