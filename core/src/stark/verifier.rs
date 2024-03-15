use crate::air::MachineAir;
use crate::stark::MachineChip;
use crate::stark::SuperChallenge;
use itertools::izip;
use itertools::Itertools;
use p3_air::Air;
use p3_air::BaseAir;
use p3_challenger::CanObserve;
use p3_challenger::FieldChallenger;
use p3_commit::UnivariatePcs;
use p3_field::AbstractExtensionField;
use p3_field::AbstractField;
use p3_field::Field;
use p3_field::TwoAdicField;
use p3_matrix::Dimensions;

use p3_util::reverse_slice_index_bits;
use std::fmt::Formatter;
use std::marker::PhantomData;

use super::folder::VerifierConstraintFolder;
use super::types::*;
use super::StarkGenericConfig;

use core::fmt::Display;

pub struct Verifier<SC, A>(PhantomData<SC>, PhantomData<A>);

impl<SC: StarkGenericConfig, A: MachineAir<SC::Val>> Verifier<SC, A> {
    /// Verify a proof for a collection of air chips.
    #[cfg(feature = "perf")]
    pub fn verify_shard(
        config: &SC,
        chips: &[&MachineChip<SC, A>],
        challenger: &mut SC::Challenger,
        proof: &ShardProof<SC>,
    ) -> Result<(), VerificationError>
    where
        A: for<'a> Air<
            VerifierConstraintFolder<
                'a,
                SC::Val,
                SuperChallenge<SC::Val>,
                SC::Val,
                SuperChallenge<SC::Val>,
                SC::Val,
                SuperChallenge<SC::Val>,
            >,
        >,
    {
        let ShardProof {
            commitment,
            opened_values,
            opening_proof,
            ..
        } = proof;

        let (main_dims, perm_dims, quot_dims): (Vec<_>, Vec<_>, Vec<_>) = chips
            .iter()
            .zip(opened_values.chips.iter())
            .map(|(chip, val)| {
                (
                    Dimensions {
                        width: chip.width(),
                        height: 1 << val.log_degree,
                    },
                    Dimensions {
                        width: (chip.sends().len() + chip.receives().len()) * 4,
                        height: 1 << val.log_degree,
                    },
                    Dimensions {
                        width: 4 << chip.log_quotient_degree(),
                        height: 1 << val.log_degree,
                    },
                )
            })
            .multiunzip();

        let dims = &[main_dims, perm_dims, quot_dims];

        let g_subgroups = opened_values
            .chips
            .iter()
            .map(|val| SC::Val::two_adic_generator(val.log_degree))
            .collect::<Vec<_>>();

        let ShardCommitment {
            main_commit,
            permutation_commit,
            quotient_commit,
        } = commitment;

        let permutation_challenges = (0..2)
            .map(|_| challenger.sample_ext_element::<SuperChallenge<SC::Val>>())
            .collect::<Vec<_>>();

        #[cfg(feature = "perf")]
        challenger.observe(permutation_commit.clone());

        let alpha = challenger.sample_ext_element::<SuperChallenge<SC::Val>>();

        // Observe the quotient commitments.
        challenger.observe(quotient_commit.clone());

        let zeta = challenger.sample_ext_element::<SuperChallenge<SC::Val>>();

        // Verify the opening proof.
        let trace_opening_points = g_subgroups
            .iter()
            .map(|g| vec![zeta, zeta * *g])
            .collect::<Vec<_>>();

        let quotient_opening_points = chips
            .iter()
            .map(|chip| vec![zeta.exp_power_of_2(chip.log_quotient_degree())])
            .collect::<Vec<_>>();

        config
            .pcs()
            .verify_multi_batches(
                &[
                    (main_commit.clone(), &trace_opening_points),
                    (permutation_commit.clone(), &trace_opening_points),
                    (quotient_commit.clone(), &quotient_opening_points),
                ],
                dims,
                opened_values.clone().into_values(),
                opening_proof,
                challenger,
            )
            .map_err(|_| VerificationError::InvalidopeningArgument)?;

        // Verify the constrtaint evaluations.

        for (chip, values, g) in izip!(chips.iter(), opened_values.chips.iter(), g_subgroups.iter())
        {
            Self::verify_constraints(
                chip,
                values.clone(),
                *g,
                zeta,
                alpha,
                &permutation_challenges,
            )
            .map_err(|_| VerificationError::OodEvaluationMismatch(chip.name()))?;
        }

        Ok(())
    }

    #[cfg(not(feature = "perf"))]
    pub fn verify_shard(
        _config: &SC,
        _chips: &[&MachineChip<SC, A>],
        _challenger: &mut SC::Challenger,
        _proof: &ShardProof<SC>,
    ) -> Result<(), VerificationError> {
        Ok(())
    }

    #[cfg(feature = "perf")]
    fn verify_constraints(
        chip: &MachineChip<SC, A>,
        opening: ChipOpenedValues<SuperChallenge<SC::Val>>,
        g: SC::Val,
        zeta: SuperChallenge<SC::Val>,
        alpha: SuperChallenge<SC::Val>,
        permutation_challenges: &[SuperChallenge<SC::Val>],
    ) -> Result<(), OodEvaluationMismatch>
    where
        A: for<'a> Air<
            VerifierConstraintFolder<
                'a,
                SC::Val,
                SuperChallenge<SC::Val>,
                SC::Val,
                SuperChallenge<SC::Val>,
                SC::Val,
                SuperChallenge<SC::Val>,
            >,
        >,
    {
        let z_h = zeta.exp_power_of_2(opening.log_degree) - SuperChallenge::<SC::Val>::one();
        let is_first_row = z_h / (zeta - SC::Val::one());
        let is_last_row = z_h / (zeta - g.inverse());
        let is_transition = zeta - g.inverse();

        // Reconstruct the prmutation opening values as extention elements.
        let monomials: Vec<SuperChallenge<SC::Val>> = (0..4)
            .map(<SuperChallenge<SC::Val> as AbstractExtensionField<SC::Val>>::monomial)
            .collect::<Vec<_>>();

        let unflatten = |v: &[SuperChallenge<SC::Val>]| {
            v.chunks_exact(4)
                .map(|chunk| {
                    chunk
                        .iter()
                        .zip(monomials.iter())
                        .map(|(x, m)| *x * *m)
                        .sum()
                })
                .collect::<Vec<SuperChallenge<SC::Val>>>()
        };

        let mut quotient_parts = opening
            .quotient
            .chunks_exact(4)
            .map(|chunk| {
                chunk
                    .iter()
                    .zip(monomials.iter())
                    .map(|(x, m)| *x * *m)
                    .sum()
            })
            .collect::<Vec<SuperChallenge<SC::Val>>>();

        reverse_slice_index_bits(&mut quotient_parts);
        let quotient: SuperChallenge<SC::Val> = zeta
            .powers()
            .zip(quotient_parts)
            .map(|(weight, part)| part * weight)
            .sum();

        let perm_opening = AirOpenedValues {
            local: unflatten(&opening.permutation.local),
            next: unflatten(&opening.permutation.next),
        };

        let mut folder = VerifierConstraintFolder::<
            SC::Val,
            SuperChallenge<SC::Val>,
            SC::Val,
            SuperChallenge<SC::Val>,
            SC::Val,
            SuperChallenge<SC::Val>,
        > {
            preprocessed: opening.preprocessed.view(),
            main: opening.main.view(),
            perm: perm_opening.view(),
            perm_challenges: permutation_challenges,
            cumulative_sum: opening.cumulative_sum,
            is_first_row,
            is_last_row,
            is_transition,
            alpha,
            accumulator: SuperChallenge::<SC::Val>::zero(),
            phantom: PhantomData,
        };
        chip.eval(&mut folder);

        let folded_constraints = folder.accumulator;

        match folded_constraints == z_h * quotient {
            true => Ok(()),
            false => Err(OodEvaluationMismatch),
        }
    }
}

pub struct OodEvaluationMismatch;

#[derive(Debug)]
pub enum VerificationError {
    /// opening proof is invalid.
    InvalidopeningArgument,
    /// Out-of-domain evaluation mismatch.
    ///
    /// `constraints(zeta)` did not match `quotient(zeta) Z_H(zeta)`.
    OodEvaluationMismatch(String),
}

impl Display for VerificationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            VerificationError::InvalidopeningArgument => {
                write!(f, "Invalid opening argument")
            }
            VerificationError::OodEvaluationMismatch(chip) => {
                write!(f, "Out-of-domain evaluation mismatch on chip {}", chip)
            }
        }
    }
}
