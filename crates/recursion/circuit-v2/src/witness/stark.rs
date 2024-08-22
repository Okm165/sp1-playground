use std::borrow::Borrow;

use p3_baby_bear::BabyBear;
use p3_field::{AbstractExtensionField, AbstractField};
use p3_fri::{CommitPhaseProofStep, QueryProof};

use sp1_recursion_compiler::ir::{Builder, Config, Ext, Felt};
use sp1_recursion_core_v2::air::Block;
use sp1_stark::{
    baby_bear_poseidon2::BabyBearPoseidon2, AirOpenedValues, ChipOpenedValues, InnerBatchOpening,
    InnerChallenge, InnerChallengeMmcs, InnerDigest, InnerFriProof, InnerPcsProof, InnerVal,
    ShardCommitment, ShardOpenedValues, ShardProof,
};

use crate::{
    stark::ShardProofVariable, BatchOpeningVariable, CircuitConfig,
    FriCommitPhaseProofStepVariable, FriProofVariable, FriQueryProofVariable,
    TwoAdicPcsProofVariable,
};

use super::{WitnessWriter, Witnessable};

pub type WitnessBlock<C> = Block<<C as Config>::F>;

impl<C: CircuitConfig<F = BabyBear, Bit = Felt<BabyBear>>> WitnessWriter<C>
    for Vec<WitnessBlock<C>>
{
    fn write_bit(&mut self, value: bool) {
        self.push(Block::from(C::F::from_bool(value)))
    }

    fn write_felt(&mut self, value: <C>::F) {
        self.push(Block::from(value))
    }

    fn write_ext(&mut self, value: <C>::EF) {
        self.push(Block::from(value.as_base_slice()))
    }
}

// TODO Bn254Fr

impl<C: CircuitConfig<F = InnerVal, EF = InnerChallenge, Bit = Felt<BabyBear>>> Witnessable<C>
    for ShardProof<BabyBearPoseidon2>
{
    type WitnessVariable = ShardProofVariable<C, BabyBearPoseidon2>;

    fn read(&self, builder: &mut Builder<C>) -> Self::WitnessVariable {
        let commitment = self.commitment.read(builder);
        let opened_values = self.opened_values.read(builder);
        let opening_proof = self.opening_proof.read(builder);
        let public_values = self.public_values.read(builder);
        let chip_ordering = self.chip_ordering.clone();

        ShardProofVariable {
            commitment,
            opened_values,
            opening_proof,
            public_values,
            chip_ordering,
        }
    }

    fn write(&self, witness: &mut impl WitnessWriter<C>) {
        self.commitment.write(witness);
        self.opened_values.write(witness);
        self.opening_proof.write(witness);
        self.public_values.write(witness);
    }
}

impl<C: CircuitConfig, T: Witnessable<C>> Witnessable<C> for ShardCommitment<T> {
    type WitnessVariable = ShardCommitment<T::WitnessVariable>;

    fn read(&self, builder: &mut Builder<C>) -> Self::WitnessVariable {
        let main_commit = self.main_commit.read(builder);
        let permutation_commit = self.permutation_commit.read(builder);
        let quotient_commit = self.quotient_commit.read(builder);
        Self::WitnessVariable { main_commit, permutation_commit, quotient_commit }
    }

    fn write(&self, witness: &mut impl WitnessWriter<C>) {
        self.main_commit.write(witness);
        self.permutation_commit.write(witness);
        self.quotient_commit.write(witness);
    }
}

impl<C: CircuitConfig<F = InnerVal, EF = InnerChallenge>> Witnessable<C>
    for ShardOpenedValues<InnerChallenge>
{
    type WitnessVariable = ShardOpenedValues<Ext<C::F, C::EF>>;

    fn read(&self, builder: &mut Builder<C>) -> Self::WitnessVariable {
        let chips = self.chips.read(builder);
        Self::WitnessVariable { chips }
    }

    fn write(&self, witness: &mut impl WitnessWriter<C>) {
        self.chips.write(witness);
    }
}

impl<C: CircuitConfig<F = InnerVal, EF = InnerChallenge>> Witnessable<C>
    for ChipOpenedValues<InnerChallenge>
{
    type WitnessVariable = ChipOpenedValues<Ext<C::F, C::EF>>;

    fn read(&self, builder: &mut Builder<C>) -> Self::WitnessVariable {
        let preprocessed = self.preprocessed.read(builder);
        let main = self.main.read(builder);
        let permutation = self.permutation.read(builder);
        let quotient = self.quotient.read(builder);
        let cumulative_sum = self.cumulative_sum.read(builder);
        let log_degree = self.log_degree;
        Self::WitnessVariable {
            preprocessed,
            main,
            permutation,
            quotient,
            cumulative_sum,
            log_degree,
        }
    }

    fn write(&self, witness: &mut impl WitnessWriter<C>) {
        self.preprocessed.write(witness);
        self.main.write(witness);
        self.permutation.write(witness);
        self.quotient.write(witness);
        self.cumulative_sum.write(witness);
    }
}

impl<C: CircuitConfig<F = InnerVal, EF = InnerChallenge>> Witnessable<C>
    for AirOpenedValues<InnerChallenge>
{
    type WitnessVariable = AirOpenedValues<Ext<C::F, C::EF>>;

    fn read(&self, builder: &mut Builder<C>) -> Self::WitnessVariable {
        let local = self.local.read(builder);
        let next = self.next.read(builder);
        Self::WitnessVariable { local, next }
    }

    fn write(&self, witness: &mut impl WitnessWriter<C>) {
        self.local.write(witness);
        self.next.write(witness);
    }
}

impl<C: CircuitConfig<F = InnerVal, EF = InnerChallenge, Bit = Felt<BabyBear>>> Witnessable<C>
    for InnerPcsProof
{
    type WitnessVariable = TwoAdicPcsProofVariable<C, BabyBearPoseidon2>;

    fn read(&self, builder: &mut Builder<C>) -> Self::WitnessVariable {
        let fri_proof = self.fri_proof.read(builder);
        let query_openings = self.query_openings.read(builder);
        Self::WitnessVariable { fri_proof, query_openings }
    }

    fn write(&self, witness: &mut impl WitnessWriter<C>) {
        self.fri_proof.write(witness);
        self.query_openings.write(witness);
    }
}

impl<C: CircuitConfig<F = InnerVal, EF = InnerChallenge, Bit = Felt<BabyBear>>> Witnessable<C>
    for InnerBatchOpening
{
    type WitnessVariable = BatchOpeningVariable<C, BabyBearPoseidon2>;

    fn read(&self, builder: &mut Builder<C>) -> Self::WitnessVariable {
        let opened_values = self
            .opened_values
            .read(builder)
            .into_iter()
            .map(|a| a.into_iter().map(|b| vec![b]).collect())
            .collect();
        let opening_proof = self.opening_proof.read(builder);
        Self::WitnessVariable { opened_values, opening_proof }
    }

    fn write(&self, witness: &mut impl WitnessWriter<C>) {
        self.opened_values.write(witness);
        self.opening_proof.write(witness);
    }
}

impl<C: CircuitConfig<F = InnerVal, EF = InnerChallenge, Bit = Felt<BabyBear>>> Witnessable<C>
    for InnerFriProof
{
    type WitnessVariable = FriProofVariable<C, BabyBearPoseidon2>;

    fn read(&self, builder: &mut Builder<C>) -> Self::WitnessVariable {
        let commit_phase_commits = self
            .commit_phase_commits
            .iter()
            .map(|commit| {
                let commit: &InnerDigest = commit.borrow();
                commit.read(builder)
            })
            .collect();
        let query_proofs = self.query_proofs.read(builder);
        let final_poly = self.final_poly.read(builder);
        let pow_witness = self.pow_witness.read(builder);
        Self::WitnessVariable { commit_phase_commits, query_proofs, final_poly, pow_witness }
    }

    fn write(&self, witness: &mut impl WitnessWriter<C>) {
        self.commit_phase_commits.iter().for_each(|commit| {
            let commit = Borrow::<InnerDigest>::borrow(commit);
            commit.write(witness);
        });
        self.query_proofs.write(witness);
        self.final_poly.write(witness);
        self.pow_witness.write(witness);
    }
}

impl<C: CircuitConfig<F = InnerVal, EF = InnerChallenge, Bit = Felt<BabyBear>>> Witnessable<C>
    for QueryProof<InnerChallenge, InnerChallengeMmcs>
{
    type WitnessVariable = FriQueryProofVariable<C, BabyBearPoseidon2>;

    fn read(&self, builder: &mut Builder<C>) -> Self::WitnessVariable {
        let commit_phase_openings = self.commit_phase_openings.read(builder);
        Self::WitnessVariable { commit_phase_openings }
    }

    fn write(&self, witness: &mut impl WitnessWriter<C>) {
        self.commit_phase_openings.write(witness);
    }
}

impl<C: CircuitConfig<F = InnerVal, EF = InnerChallenge, Bit = Felt<BabyBear>>> Witnessable<C>
    for CommitPhaseProofStep<InnerChallenge, InnerChallengeMmcs>
{
    type WitnessVariable = FriCommitPhaseProofStepVariable<C, BabyBearPoseidon2>;

    fn read(&self, builder: &mut Builder<C>) -> Self::WitnessVariable {
        let sibling_value = self.sibling_value.read(builder);
        let opening_proof = self.opening_proof.read(builder);
        Self::WitnessVariable { sibling_value, opening_proof }
    }

    fn write(&self, witness: &mut impl WitnessWriter<C>) {
        self.sibling_value.write(witness);
        self.opening_proof.write(witness);
    }
}
