use std::{array, sync::Arc};

use p3_field::{AbstractField, PrimeField32};
use sp1_core::{
    stark::{MachineRecord, PROOF_MAX_NUM_PVS},
    utils::SP1CoreOpts,
};
use sp1_recursion_core::air::RecursionPublicValues;

// TODO expand glob imports
use crate::*;

#[derive(Clone, Default, Debug)]
pub struct ExecutionRecord<F> {
    pub program: Arc<RecursionProgram<F>>,
    /// The index of the shard.
    pub index: u32,

    pub base_alu_events: Vec<BaseAluEvent<F>>,
    pub ext_alu_events: Vec<ExtAluEvent<F>>,
    pub mem_events: Vec<MemEvent<F>>,
    /// The public values.
    pub public_values: RecursionPublicValues<F>,

    pub poseidon2_skinny_events: Vec<Poseidon2SkinnyEvent<F>>,
    pub poseidon2_wide_events: Vec<Poseidon2WideEvent<F>>,
    pub exp_reverse_bits_len_events: Vec<ExpReverseBitsEvent<F>>,
    pub fri_fold_events: Vec<FriFoldEvent<F>>,
    pub commit_pv_hash_events: Vec<CommitPublicValuesEvent<F>>,
}

impl<F: PrimeField32> MachineRecord for ExecutionRecord<F> {
    type Config = SP1CoreOpts;

    fn stats(&self) -> hashbrown::HashMap<String, usize> {
        hashbrown::HashMap::from([("cpu_events".to_owned(), 1337usize)])
    }

    fn append(&mut self, other: &mut Self) {
        // Exhaustive destructuring for refactoring purposes.
        let Self {
            program: _,
            index: _,
            base_alu_events,
            ext_alu_events,
            mem_events,
            public_values: _,
            poseidon2_wide_events,
            poseidon2_skinny_events,
            exp_reverse_bits_len_events,
            fri_fold_events,
            commit_pv_hash_events,
        } = self;
        base_alu_events.append(&mut other.base_alu_events);
        ext_alu_events.append(&mut other.ext_alu_events);
        mem_events.append(&mut other.mem_events);
        poseidon2_wide_events.append(&mut other.poseidon2_wide_events);
        poseidon2_skinny_events.append(&mut other.poseidon2_skinny_events);
        exp_reverse_bits_len_events.append(&mut other.exp_reverse_bits_len_events);
        fri_fold_events.append(&mut other.fri_fold_events);
        commit_pv_hash_events.append(&mut other.commit_pv_hash_events);
    }

    fn public_values<T: AbstractField>(&self) -> Vec<T> {
        let pv_elms = self.public_values.to_vec();

        let ret: [T; PROOF_MAX_NUM_PVS] = array::from_fn(|i| {
            if i < pv_elms.len() {
                T::from_canonical_u32(pv_elms[i].as_canonical_u32())
            } else {
                T::zero()
            }
        });

        ret.to_vec()
    }
}
