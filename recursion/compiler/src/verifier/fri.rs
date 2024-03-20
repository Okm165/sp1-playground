use super::challenger::DuplexChallenger;
use super::types::Dimensions;
use super::types::FriChallenges;
use super::types::FriConfig;
use super::types::FriProof;
use super::types::FriQueryProof;
use super::types::DIGEST_SIZE;
use super::types::PERMUTATION_WIDTH;
use crate::prelude::Array;
use crate::prelude::Builder;
use crate::prelude::Config;
use crate::prelude::DslIR;
use crate::prelude::Felt;
use crate::prelude::SymbolicVar;
use crate::prelude::Usize;
use crate::prelude::Var;
use crate::verifier::types::Commitment;

use p3_field::AbstractField;
use p3_field::TwoAdicField;

impl<C: Config> Builder<C> {
    pub fn error(&mut self) {
        self.operations.push(DslIR::Error());
    }

    /// Converts a usize to a fixed length of bits.
    pub fn num2bits_v(&mut self, num: Var<C::N>) -> Array<C, Var<C::N>> {
        let output = self.array::<Var<_>, _>(Usize::Const(29));
        self.operations
            .push(DslIR::Num2BitsV(output.clone(), Usize::Var(num)));
        output
    }

    /// Converts a felt to a fixed length of bits.
    pub fn num2bits_f(&mut self, num: Felt<C::F>) -> Array<C, Var<C::N>> {
        let output = self.array::<Var<_>, _>(Usize::Const(29));
        self.operations.push(DslIR::Num2BitsF(output.clone(), num));
        output
    }

    /// Applies the Poseidon2 permutation to the given array.
    ///
    /// Reference: https://github.com/Plonky3/Plonky3/blob/4809fa7bedd9ba8f6f5d3267b1592618e3776c57/poseidon2/src/lib.rs#L119
    pub fn poseidon2_permute(&mut self, array: &Array<C, Felt<C::F>>) -> Array<C, Felt<C::F>> {
        let output = match array {
            Array::Fixed(values) => {
                assert_eq!(values.len(), PERMUTATION_WIDTH);
                self.array::<Felt<C::F>, _>(Usize::Const(PERMUTATION_WIDTH))
            }
            Array::Dyn(_, len) => self.array::<Felt<C::F>, _>(*len),
        };
        self.operations
            .push(DslIR::Poseidon2Permute(output.clone(), array.clone()));
        output
    }

    /// Applies the Poseidon2 compression function to the given array.
    ///
    /// Assumes we are doing a 2-1 compression function with 8 element chunks.
    ///
    /// Reference: https://github.com/Plonky3/Plonky3/blob/4809fa7bedd9ba8f6f5d3267b1592618e3776c57/symmetric/src/compression.rs#L35
    pub fn poseidon2_compress(
        &mut self,
        left: &Array<C, Felt<C::F>>,
        right: &Array<C, Felt<C::F>>,
    ) -> Array<C, Felt<C::F>> {
        let output = match left {
            Array::Fixed(values) => {
                assert_eq!(values.len(), DIGEST_SIZE);
                self.array::<Felt<C::F>, _>(Usize::Const(DIGEST_SIZE))
            }
            Array::Dyn(_, _) => {
                let len: Var<C::N> = self.eval(C::N::from_canonical_usize(DIGEST_SIZE));
                self.array::<Felt<C::F>, _>(Usize::Var(len))
            }
        };
        self.operations.push(DslIR::Poseidon2Compress(
            output.clone(),
            left.clone(),
            right.clone(),
        ));
        output
    }

    /// Applies the Poseidon2 hash function to the given array using a padding-free sponge.
    ///
    /// Reference: https://github.com/Plonky3/Plonky3/blob/4809fa7bedd9ba8f6f5d3267b1592618e3776c57/symmetric/src/sponge.rs#L32
    pub fn poseidon2_hash(&mut self, input: Array<C, Felt<C::F>>) -> Array<C, Felt<C::F>> {
        let len = match input {
            Array::Fixed(_) => Usize::Const(PERMUTATION_WIDTH),
            Array::Dyn(_, _) => {
                let len: Var<_> = self.eval(C::N::from_canonical_usize(PERMUTATION_WIDTH));
                Usize::Var(len)
            }
        };
        let state = self.array::<Felt<C::F>, _>(len);
        let start: Usize<C::N> = Usize::Const(0);
        let end = len;
        self.range(start, end).for_each(|_, builder| {
            let new_state = builder.poseidon2_permute(&state);
            builder.assign(state.clone(), new_state);
        });
        state
    }

    /// Materializes a usize into a variable.
    pub fn materialize(&mut self, num: Usize<C::N>) -> Var<C::N> {
        match num {
            Usize::Const(num) => self.eval(C::N::from_canonical_usize(num)),
            Usize::Var(num) => num,
        }
    }

    /// Reference: https://github.com/Plonky3/Plonky3/blob/4809fa7bedd9ba8f6f5d3267b1592618e3776c57/baby-bear/src/baby_bear.rs#L306
    pub fn generator(&mut self) -> Felt<C::F> {
        self.eval(C::F::from_canonical_u32(0x78000000))
    }

    /// Reference: https://github.com/Plonky3/Plonky3/blob/4809fa7bedd9ba8f6f5d3267b1592618e3776c57/baby-bear/src/baby_bear.rs#L302
    #[allow(unused_variables)]
    pub fn two_adic_generator(&mut self, bits: Usize<C::N>) -> Felt<C::F> {
        let result = self.uninit();
        self.operations.push(DslIR::TwoAdicGenerator(result, bits));
        result
    }

    /// Reference: https://github.com/Plonky3/Plonky3/blob/4809fa7bedd9ba8f6f5d3267b1592618e3776c57/util/src/lib.rs#L59
    #[allow(unused_variables)]
    pub fn reverse_bits_len(&mut self, index: Var<C::N>, bit_len: Usize<C::N>) -> Usize<C::N> {
        let result = self.uninit();
        self.operations
            .push(DslIR::ReverseBitsLen(result, Usize::Var(index), bit_len));
        result
    }

    /// Reference: https://github.com/Plonky3/Plonky3/blob/4809fa7bedd9ba8f6f5d3267b1592618e3776c57/field/src/field.rs#L79
    #[allow(unused_variables)]
    pub fn exp_usize_f(&mut self, x: Felt<C::F>, power: Usize<C::N>) -> Felt<C::F> {
        let result = self.uninit();
        self.operations.push(DslIR::ExpUsizeF(result, x, power));
        result
    }

    /// Reference: https://github.com/Plonky3/Plonky3/blob/4809fa7bedd9ba8f6f5d3267b1592618e3776c57/field/src/field.rs#L79
    #[allow(unused_variables)]
    pub fn exp_usize_v(&mut self, x: Var<C::N>, power: Usize<C::N>) -> Var<C::N> {
        let result = self.uninit();
        self.operations.push(DslIR::ExpUsizeV(result, x, power));
        result
    }
}

/// Reference: https://github.com/Plonky3/Plonky3/blob/4809fa7bedd9ba8f6f5d3267b1592618e3776c57/fri/src/verifier.rs#L27
pub fn verify_shape_and_sample_challenges<C: Config>(
    builder: &mut Builder<C>,
    config: &FriConfig<C>,
    proof: &FriProof<C>,
    challenger: &mut DuplexChallenger<C>,
) -> FriChallenges<C> {
    let mut betas: Array<C, Felt<C::F>> = builder.array(proof.commit_phase_commits.len());

    builder
        .range(0, proof.commit_phase_commits.len())
        .for_each(|i, builder| {
            let comm = builder.get(&proof.commit_phase_commits, i);
            challenger.observe_commitment(builder, comm);

            let sample = challenger.sample(builder);
            builder.set(&mut betas, i, sample);
        });

    let num_commit_phase_commits = proof.commit_phase_commits.len().materialize(builder);
    let num_queries = config.num_queries.materialize(builder);
    builder
        .if_ne(num_commit_phase_commits, num_queries)
        .then(|builder| {
            builder.error();
        });

    // TODO: Check PoW.
    // if !challenger.check_witness(config.proof_of_work_bits, proof.pow_witness) {
    //     return Err(FriError::InvalidPowWitness);
    // }

    let log_blowup = config.log_blowup.materialize(builder);
    let log_max_height: Var<_> = builder.eval(num_commit_phase_commits + log_blowup);
    let mut query_indices = builder.array(config.num_queries);
    builder.range(0, config.num_queries).for_each(|i, builder| {
        let index = challenger.sample_bits(builder, Usize::Var(log_max_height));
        builder.set(&mut query_indices, i, index);
    });

    FriChallenges {
        query_indices,
        betas,
    }
}

/// Verifies a set of FRI challenges.
///
/// Reference: https://github.com/Plonky3/Plonky3/blob/4809fa7bedd9ba8f6f5d3267b1592618e3776c57/fri/src/verifier.rs#L67
#[allow(unused_variables)]
pub fn verify_challenges<C: Config>(
    builder: &mut Builder<C>,
    config: &FriConfig<C>,
    proof: &FriProof<C>,
    challenges: &FriChallenges<C>,
    reduced_openings: &Array<C, Array<C, Felt<C::F>>>,
) where
    C::F: TwoAdicField,
{
    let nb_commit_phase_commits = proof.commit_phase_commits.len().materialize(builder);
    let log_blowup = config.log_blowup.materialize(builder);
    let log_max_height = builder.eval(nb_commit_phase_commits + log_blowup);
    builder
        .range(0, challenges.query_indices.len())
        .for_each(|i, builder| {
            let index = builder.get(&challenges.query_indices, i);
            let query_proof = builder.get(&proof.query_proofs, i);
            let ro = builder.get(reduced_openings, i);

            let folded_eval = verify_query(
                builder,
                config,
                &proof.commit_phase_commits,
                index,
                &query_proof,
                &challenges.betas,
                &ro,
                Usize::Var(log_max_height),
            );

            builder.assert_felt_eq(folded_eval, proof.final_poly);
        });
}

/// Verifies a FRI query.
///
/// Currently assumes the index that is accessed is constant.
///
/// Reference: https://github.com/Plonky3/Plonky3/blob/4809fa7bedd9ba8f6f5d3267b1592618e3776c57/fri/src/verifier.rs#L101
#[allow(clippy::too_many_arguments)]
#[allow(unused_variables)]
pub fn verify_query<C: Config>(
    builder: &mut Builder<C>,
    config: &FriConfig<C>,
    commit_phase_commits: &Array<C, Commitment<C>>,
    index: Var<C::N>,
    proof: &FriQueryProof<C>,
    betas: &Array<C, Felt<C::F>>,
    reduced_openings: &Array<C, Felt<C::F>>,
    log_max_height: Usize<C::N>,
) -> Felt<C::F>
where
    C::F: TwoAdicField,
{
    let folded_eval: Felt<_> = builder.eval(C::F::zero());
    let two_adic_generator = builder.two_adic_generator(log_max_height);
    let power = builder.reverse_bits_len(index, log_max_height);
    let x = builder.exp_usize_f(two_adic_generator, power);

    let index_bits = builder.num2bits_v(index);

    let log_max_height = log_max_height.materialize(builder);
    builder.range(0, log_max_height).for_each(|i, builder| {
        let log_folded_height: Var<_> = builder.eval(log_max_height - i - C::N::one());
        let log_folded_height_plus_one: Var<_> = builder.eval(log_max_height - i);
        let commit = builder.get(commit_phase_commits, i);
        let step = builder.get(&proof.commit_phase_openings, i);
        let beta = builder.get(betas, i);

        let index_bit = builder.get(&index_bits, i);
        let index_sibling_mod_2: Var<C::N> =
            builder.eval(SymbolicVar::Const(C::N::one()) - index_bit);
        let i_plus_one = builder.eval(i + C::N::one());
        let index_pair = index_bits.shift(builder, i_plus_one);

        let mut evals: Array<C, Felt<C::F>> = builder.array(2);
        builder.set(&mut evals, index_sibling_mod_2, step.sibling_value);

        let two: Var<C::N> = builder.eval(C::N::from_canonical_u32(2));
        let dims = Dimensions::<C> {
            height: builder.exp_usize_v(two, Usize::Var(log_folded_height)),
        };
        let mut dims_slice: Array<C, Dimensions<C>> = builder.array(1);
        builder.set(&mut dims_slice, 0, dims);

        let mut opened_values = builder.array(1);
        builder.set(&mut opened_values, 0, evals);
        verify_batch(
            builder,
            &commit,
            dims_slice,
            index_pair,
            opened_values,
            &step.opening_proof,
        );

        let mut xs: Array<C, Felt<C::F>> = builder.array(2);
        let two_adic_generator_one = builder.two_adic_generator(Usize::Const(1));
        builder.set(&mut xs, 0, x);
        builder.set(&mut xs, 1, x);
        builder.set(&mut xs, index_sibling_mod_2, two_adic_generator_one);

        let xs_0 = builder.get(&xs, 0);
        let xs_1 = builder.get(&xs, 1);
        let eval_0 = builder.get(&evals, 0);
        let eval_1 = builder.get(&evals, 1);
        builder.assign(
            folded_eval,
            eval_0 + (beta - xs_0) * (eval_1 - eval_0) / (xs_1 - xs_0),
        );

        builder.assign(x, x * x);
    });

    // debug_assert!(index < config.blowup(), "index was {}", index);
    // debug_assert_eq!(x.exp_power_of_2(config.log_blowup), F::one());

    folded_eval
}

/// Verifies a batch opening.
///
/// Assumes the dimensions have already been sorted.
///
/// Reference: https://github.com/Plonky3/Plonky3/blob/4809fa7bedd9ba8f6f5d3267b1592618e3776c57/merkle-tree/src/mmcs.rs#L92
#[allow(unused_variables)]
pub fn verify_batch<C: Config>(
    builder: &mut Builder<C>,
    commit: &Commitment<C>,
    dims: Array<C, Dimensions<C>>,
    index_bits: Array<C, Var<C::N>>,
    opened_values: Array<C, Array<C, Felt<C::F>>>,
    proof: &Array<C, Commitment<C>>,
) {
    // let curr_height_padded: Var<C::N> =
    //     builder.eval(dims[0].height * C::N::from_canonical_usize(2));

    // let two: Var<_> = builder.eval(C::N::from_canonical_u32(2));
    // let array = builder.array::<Felt<_>, _>(Usize::Var(two));
    // let root = builder.poseidon2_hash(array);

    // let start = Usize::Const(0);
    // let end = proof.len();
    // let index_bits = builder.num2bits_v(Usize::Const(index));
    // builder.range(start, end).for_each(|i, builder| {
    //     let bit = builder.get(&index_bits, i);
    //     let left: Array<C, Felt<C::F>> = builder.uninit();
    //     let right: Array<C, Felt<C::F>> = builder.uninit();
    //     let one: Var<_> = builder.eval(C::N::one());
    //     let sibling = builder.get(proof, i);
    //     builder.if_eq(bit, one).then_or_else(
    //         |builder| {
    //             builder.assign(left.clone(), root.clone());
    //             builder.assign(right.clone(), sibling.clone());
    //         },
    //         |builder| {
    //             builder.assign(left.clone(), sibling.clone());
    //             builder.assign(right.clone(), root.clone());
    //         },
    //     );

    //     let new_root = builder.poseidon2_compress(&left, &right);
    //     builder.assign(root.clone(), new_root);
    // });

    // let start = Usize::Const(0);
    // let end = Usize::Const(DIGEST_SIZE);
    // builder.range(start, end).for_each(|i, builder| {
    //     let lhs = builder.get(commit, i);
    //     let rhs = builder.get(&root, i);
    //     builder.assert_felt_eq(lhs, rhs);
    // })
}
