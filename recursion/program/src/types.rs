use p3_air::BaseAir;
use p3_field::{AbstractExtensionField, AbstractField};
use sp1_core::{
    air::{MachineAir, PublicValues, Word, PV_DIGEST_NUM_WORDS, WORD_SIZE},
    stark::{AirOpenedValues, Chip, ChipOpenedValues},
};
use sp1_recursion_compiler::{prelude::*, InnerConfig};
use sp1_recursion_core::runtime::DIGEST_SIZE;

use crate::fri::types::TwoAdicPcsProofVariable;
use crate::fri::types::{DigestVariable, FriConfigVariable};
use crate::fri::TwoAdicMultiplicativeCosetVariable;

#[derive(DslVariable, Clone)]
pub struct PublicValuesVariable<C: Config> {
    pub committed_values_digest: Array<C, Felt<C::F>>,
    pub shard: Felt<C::F>,
    pub start_pc: Felt<C::F>,
    pub next_pc: Felt<C::F>,
    pub exit_code: Felt<C::F>,
}

impl<C: Config> PublicValuesVariable<C> {
    pub fn to_vec(&self, builder: &mut Builder<C>) -> Vec<Felt<C::F>> {
        let mut result = Vec::new();

        for i in 0..PV_DIGEST_NUM_WORDS {
            for j in 0..WORD_SIZE {
                let el = builder.get(&self.committed_values_digest, i * WORD_SIZE + j);
                result.push(el);
            }
        }

        result.push(self.shard);
        result.push(self.start_pc);
        result.push(self.next_pc);
        result.push(self.exit_code);

        result
    }
}

impl<C: Config> FromConstant<C> for PublicValuesVariable<C> {
    type Constant = PublicValues<u32, u32>;

    fn eval_const(value: Self::Constant, builder: &mut Builder<C>) -> Self {
        let pv_shard = builder.eval(C::F::from_canonical_u32(value.shard));
        let pv_start_pc = builder.eval(C::F::from_canonical_u32(value.start_pc));
        let pv_next_pc = builder.eval(C::F::from_canonical_u32(value.next_pc));
        let pv_exit_code = builder.eval(C::F::from_canonical_u32(value.exit_code));
        let mut pv_committed_value_digest = Vec::new();
        for i in 0..PV_DIGEST_NUM_WORDS {
            let word_val: Word<C::F> = Word::from(value.committed_value_digest[i]);
            for j in 0..WORD_SIZE {
                let word_val: Felt<_> = builder.eval(word_val[j]);
                pv_committed_value_digest.push(word_val);
            }
        }

        PublicValuesVariable {
            committed_values_digest: builder.vec(pv_committed_value_digest),
            shard: pv_shard,
            start_pc: pv_start_pc,
            next_pc: pv_next_pc,
            exit_code: pv_exit_code,
        }
    }
}

/// Reference: https://github.com/Plonky3/Plonky3/blob/4809fa7bedd9ba8f6f5d3267b1592618e3776c57/fri/src/proof.rs#L12
#[derive(DslVariable, Clone)]
pub struct ShardProofVariable<C: Config> {
    pub index: Var<C::N>,
    pub commitment: ShardCommitmentVariable<C>,
    pub opened_values: ShardOpenedValuesVariable<C>,
    pub opening_proof: TwoAdicPcsProofVariable<C>,
    pub public_values: PublicValuesVariable<C>,
}

/// Reference: https://github.com/succinctlabs/sp1/blob/b5d5473c010ab0630102652146e16c014a1eddf6/core/src/stark/machine.rs#L63
#[derive(DslVariable, Clone)]
pub struct VerifyingKeyVariable<C: Config> {
    pub commitment: DigestVariable<C>,
    // TODO: witness chip info?
}

#[derive(DslVariable, Clone)]
pub struct ShardCommitmentVariable<C: Config> {
    pub main_commit: DigestVariable<C>,
    pub permutation_commit: DigestVariable<C>,
    pub quotient_commit: DigestVariable<C>,
}

#[derive(DslVariable, Debug, Clone)]
pub struct ShardOpenedValuesVariable<C: Config> {
    pub chips: Array<C, ChipOpenedValuesVariable<C>>,
}

#[derive(Debug, Clone)]
pub struct ChipOpening<C: Config> {
    pub preprocessed: AirOpenedValues<Ext<C::F, C::EF>>,
    pub main: AirOpenedValues<Ext<C::F, C::EF>>,
    pub permutation: AirOpenedValues<Ext<C::F, C::EF>>,
    pub quotient: Vec<Vec<Ext<C::F, C::EF>>>,
    pub cumulative_sum: Ext<C::F, C::EF>,
    pub log_degree: Var<C::N>,
}

#[derive(DslVariable, Debug, Clone)]
pub struct ChipOpenedValuesVariable<C: Config> {
    pub preprocessed: AirOpenedValuesVariable<C>,
    pub main: AirOpenedValuesVariable<C>,
    pub permutation: AirOpenedValuesVariable<C>,
    pub quotient: Array<C, Array<C, Ext<C::F, C::EF>>>,
    pub cumulative_sum: Ext<C::F, C::EF>,
    pub log_degree: Var<C::N>,
}

#[derive(DslVariable, Debug, Clone)]
pub struct AirOpenedValuesVariable<C: Config> {
    pub local: Array<C, Ext<C::F, C::EF>>,
    pub next: Array<C, Ext<C::F, C::EF>>,
}

impl<C: Config> ChipOpening<C> {
    pub fn from_variable<A>(
        builder: &mut Builder<C>,
        chip: &Chip<C::F, A>,
        opening: &ChipOpenedValuesVariable<C>,
    ) -> Self
    where
        A: MachineAir<C::F>,
    {
        let mut preprocessed = AirOpenedValues {
            local: vec![],
            next: vec![],
        };

        let preprocessed_width = chip.preprocessed_width();
        for i in 0..preprocessed_width {
            preprocessed
                .local
                .push(builder.get(&opening.preprocessed.local, i));
            preprocessed
                .next
                .push(builder.get(&opening.preprocessed.next, i));
        }

        let mut main = AirOpenedValues {
            local: vec![],
            next: vec![],
        };
        let main_width = chip.width();
        for i in 0..main_width {
            main.local.push(builder.get(&opening.main.local, i));
            main.next.push(builder.get(&opening.main.next, i));
        }

        let mut permutation = AirOpenedValues {
            local: vec![],
            next: vec![],
        };
        let permutation_width = C::EF::D * (chip.num_interactions() + 1);
        for i in 0..permutation_width {
            permutation
                .local
                .push(builder.get(&opening.permutation.local, i));
            permutation
                .next
                .push(builder.get(&opening.permutation.next, i));
        }

        let num_quotient_chunks = 1 << chip.log_quotient_degree();

        let mut quotient = vec![];
        for i in 0..num_quotient_chunks {
            let chunk = builder.get(&opening.quotient, i);
            let mut quotient_vals = vec![];
            for j in 0..C::EF::D {
                let value = builder.get(&chunk, j);
                quotient_vals.push(value);
            }
            quotient.push(quotient_vals);
        }

        ChipOpening {
            preprocessed,
            main,
            permutation,
            quotient,
            cumulative_sum: opening.cumulative_sum,
            log_degree: opening.log_degree,
        }
    }
}

impl<C: Config> FromConstant<C> for AirOpenedValuesVariable<C> {
    type Constant = AirOpenedValues<C::EF>;

    fn eval_const(value: Self::Constant, builder: &mut Builder<C>) -> Self {
        AirOpenedValuesVariable {
            local: builder.eval_const(value.local),
            next: builder.eval_const(value.next),
        }
    }
}

impl<C: Config> FromConstant<C> for ChipOpenedValuesVariable<C> {
    type Constant = ChipOpenedValues<C::EF>;

    fn eval_const(value: Self::Constant, builder: &mut Builder<C>) -> Self {
        ChipOpenedValuesVariable {
            preprocessed: builder.eval_const(value.preprocessed),
            main: builder.eval_const(value.main),
            permutation: builder.eval_const(value.permutation),
            quotient: builder.eval_const(value.quotient),
            cumulative_sum: builder.eval(value.cumulative_sum.cons()),
            log_degree: builder.eval(C::N::from_canonical_usize(value.log_degree)),
        }
    }
}

impl<C: Config> FriConfigVariable<C> {
    pub fn get_subgroup(
        &self,
        builder: &mut Builder<C>,
        log_degree: impl Into<Usize<C::N>>,
    ) -> TwoAdicMultiplicativeCosetVariable<C> {
        builder.get(&self.subgroups, log_degree)
    }

    pub fn get_two_adic_generator(
        &self,
        builder: &mut Builder<C>,
        bits: impl Into<Usize<C::N>>,
    ) -> Felt<C::F> {
        builder.get(&self.generators, bits)
    }
}
