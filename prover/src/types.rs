use std::{fs::File, path::Path};

use anyhow::Result;
use p3_baby_bear::BabyBear;
use p3_commit::{Pcs, TwoAdicMultiplicativeCoset};
use p3_field::{AbstractField, PrimeField32, TwoAdicField};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sp1_core::{
    io::{SP1PublicValues, SP1Stdin},
    stark::{ShardProof, StarkGenericConfig, StarkProvingKey, StarkVerifyingKey},
    utils::DIGEST_SIZE,
};
use sp1_primitives::poseidon2_hash;
use sp1_recursion_gnark_ffi::{plonk_bn254::PlonkBn254Proof, Groth16Proof};

use crate::utils::words_to_bytes_be;
use crate::{CoreSC, InnerSC};

/// The information necessary to generate a proof for a given RISC-V program.
pub struct SP1ProvingKey {
    pub pk: StarkProvingKey<CoreSC>,
    pub elf: Vec<u8>,
    /// Verifying key is also included as we need it for recursion
    pub vk: SP1VerifyingKey,
}

/// The information necessary to verify a proof for a given RISC-V program.
#[derive(Clone)]
pub struct SP1VerifyingKey {
    pub vk: StarkVerifyingKey<CoreSC>,
}

/// A trait for keys that can be hashed into a digest.
pub trait HashableKey {
    /// Hash the key into a digest of 8 BabyBear elements.
    fn hash(&self) -> [BabyBear; 8];

    /// Hash the key into a digest of 8 u32 elements.
    fn hash_u32(&self) -> [u32; 8];

    /// Hash the key into a digest of 8 u32 elements.
    fn hash_bytes(&self) -> [u8; 32] {
        words_to_bytes_be(&self.hash_u32())
    }
}

impl HashableKey for SP1VerifyingKey {
    fn hash(&self) -> [BabyBear; 8] {
        self.vk.hash()
    }

    fn hash_u32(&self) -> [u32; 8] {
        self.vk.hash_u32()
    }
}

impl<SC: StarkGenericConfig<Val = BabyBear, Domain = TwoAdicMultiplicativeCoset<BabyBear>>>
    HashableKey for StarkVerifyingKey<SC>
where
    <SC::Pcs as Pcs<SC::Challenge, SC::Challenger>>::Commitment: AsRef<[BabyBear; DIGEST_SIZE]>,
{
    fn hash(&self) -> [BabyBear; 8] {
        let prep_domains = self.chip_information.iter().map(|(_, domain, _)| domain);
        let num_inputs = DIGEST_SIZE + 1 + (4 * prep_domains.len());
        let mut inputs = Vec::with_capacity(num_inputs);
        inputs.extend(self.commit.as_ref());
        inputs.push(self.pc_start);
        for domain in prep_domains {
            inputs.push(BabyBear::from_canonical_usize(domain.log_n));
            let size = 1 << domain.log_n;
            inputs.push(BabyBear::from_canonical_usize(size));
            let g = BabyBear::two_adic_generator(domain.log_n);
            inputs.push(domain.shift);
            inputs.push(g);
        }

        poseidon2_hash(inputs)
    }

    fn hash_u32(&self) -> [u32; 8] {
        self.hash()
            .into_iter()
            .map(|n| n.as_canonical_u32())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

/// A proof of a RISCV ELF execution with given inputs and outputs.
#[derive(Serialize, Deserialize, Clone)]
#[serde(bound(serialize = "P: Serialize"))]
#[serde(bound(deserialize = "P: DeserializeOwned"))]
pub struct SP1ProofWithMetadata<P: Clone> {
    pub proof: P,
    pub stdin: SP1Stdin,
    pub public_values: SP1PublicValues,
}

impl<P: Serialize + DeserializeOwned + Clone> SP1ProofWithMetadata<P> {
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        bincode::serialize_into(File::create(path).expect("failed to open file"), self)
            .map_err(Into::into)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        bincode::deserialize_from(File::open(path).expect("failed to open file"))
            .map_err(Into::into)
    }
}

impl<P: std::fmt::Debug + Clone> std::fmt::Debug for SP1ProofWithMetadata<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SP1ProofWithMetadata")
            .field("proof", &self.proof)
            .finish()
    }
}

/// A proof of an SP1 program without any wrapping.
pub type SP1CoreProof = SP1ProofWithMetadata<SP1CoreProofData>;

/// An SP1 proof that has been recursively reduced into a single proof. This proof can be verified
/// within SP1 programs.
pub type SP1ReducedProof = SP1ProofWithMetadata<SP1ReducedProofData>;

/// An SP1 proof that has been wrapped into a single Groth16 proof and can be verified onchain.
pub type SP1Groth16Proof = SP1ProofWithMetadata<SP1Groth16ProofData>;

/// An SP1 proof that has been wrapped into a single Plonk proof and can be verified onchain.
pub type SP1PlonkProof = SP1ProofWithMetadata<SP1PlonkProofData>;

#[derive(Serialize, Deserialize, Clone)]
pub struct SP1CoreProofData(pub Vec<ShardProof<CoreSC>>);
#[derive(Serialize, Deserialize, Clone)]
pub struct SP1ReducedProofData(pub ShardProof<InnerSC>);

#[derive(Serialize, Deserialize, Clone)]
pub struct SP1Groth16ProofData(pub Groth16Proof);

#[derive(Serialize, Deserialize, Clone)]
pub struct SP1PlonkProofData(pub PlonkBn254Proof);

/// An intermediate proof which proves the execution over a range of shards.
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "ShardProof<SC>: Serialize"))]
#[serde(bound(deserialize = "ShardProof<SC>: Deserialize<'de>"))]
pub struct SP1ReduceProof<SC: StarkGenericConfig> {
    pub proof: ShardProof<SC>,
}

/// A proof that can be reduced along with other proofs into one proof.
#[derive(Serialize, Deserialize)]
pub enum SP1ReduceProofWrapper {
    Core(SP1ReduceProof<CoreSC>),
    Recursive(SP1ReduceProof<InnerSC>),
}
