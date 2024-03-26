use p3_air::Air;
use sp1_core::stark::{MachineChip, StarkGenericConfig, VerifierConstraintFolder};
use sp1_recursion_compiler::{
    ir::{Builder, Config},
    verifier::challenger::DuplexChallengerVariable,
};

use crate::types::ShardProofVariable;

#[derive(Debug, Clone, Copy)]
pub struct StarkVerifier<C: Config, SC: StarkGenericConfig> {
    _phantom: std::marker::PhantomData<(C, SC)>,
}

impl<C: Config, SC: StarkGenericConfig> StarkVerifier<C, SC>
where
    SC: StarkGenericConfig<Val = C::F, Challenge = C::EF>,
{
    pub fn verify_shard<A>(
        &mut self,
        chips: &[&MachineChip<SC, A>],
        challenger: &mut DuplexChallengerVariable<C>,
        proof: &ShardProofVariable<C>,
    ) where
        A: for<'b> Air<VerifierConstraintFolder<'b, SC>>,
    {
        let ShardProofVariable {
            commitment,
            opened_values,
            opening_proof,
            ..
        } = proof;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use sp1_core::{
        stark::{RiscvAir, ShardCommitment, ShardProof, StarkGenericConfig},
        utils::BabyBearPoseidon2,
    };
    use sp1_recursion_compiler::{
        ir::{Builder, Config, Usize},
        verifier::fri::types::{Commitment, DIGEST_SIZE},
    };

    use crate::{
        fri::{const_fri_proof, const_two_adic_pcs_proof},
        types::{ShardOpenedValuesVariable, ShardProofVariable},
    };

    type SC = BabyBearPoseidon2;
    type F = <SC as StarkGenericConfig>::Val;
    type EF = <SC as StarkGenericConfig>::Challenge;
    type A = RiscvAir<F>;

    pub(crate) fn const_proof<C, A>(
        builder: &mut Builder<C>,
        proof: ShardProof<SC>,
    ) -> ShardProofVariable<C>
    where
        C: Config<F = F, EF = EF>,
    {
        let index = builder.materialize(Usize::Const(proof.index));

        // Set up the commitments.
        let mut main_commit: Commitment<_> = builder.dyn_array(DIGEST_SIZE);
        let mut permutation_commit: Commitment<_> = builder.dyn_array(DIGEST_SIZE);
        let mut quotient_commit: Commitment<_> = builder.dyn_array(DIGEST_SIZE);

        let main_commit_val: [_; DIGEST_SIZE] = proof.commitment.main_commit.into();
        let perm_commit_val: [_; DIGEST_SIZE] = proof.commitment.permutation_commit.into();
        let quotient_commit_val: [_; DIGEST_SIZE] = proof.commitment.quotient_commit.into();
        for (i, ((main_val, perm_val), quotient_val)) in main_commit_val
            .into_iter()
            .zip(perm_commit_val)
            .zip(quotient_commit_val)
            .enumerate()
        {
            builder.set(&mut main_commit, i, main_val);
            builder.set(&mut permutation_commit, i, perm_val);
            builder.set(&mut quotient_commit, i, quotient_val);
        }

        let commitment = ShardCommitment {
            main_commit,
            permutation_commit,
            quotient_commit,
        };

        // Set up the opened values.
        let opened_values = ShardOpenedValuesVariable {
            chips: proof
                .opened_values
                .chips
                .iter()
                .map(|values| builder.const_chip_opening(values))
                .collect(),
        };

        let opening_proof = const_two_adic_pcs_proof(builder, proof.opening_proof);

        ShardProofVariable {
            index: Usize::Var(index),
            commitment,
            opened_values,
            opening_proof,
        }
    }

    #[test]
    fn test_verify_shard() {}
}
