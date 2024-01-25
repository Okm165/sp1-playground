use crate::runtime::Runtime;

use crate::utils::Chip;
use p3_air::VirtualPairCol;
use p3_baby_bear::BabyBear;
use p3_challenger::DuplexChallenger;
use p3_commit::ExtensionMmcs;
use p3_dft::Radix2DitParallel;
use p3_field::extension::BinomialExtensionField;
use p3_field::Field;
use p3_field::PrimeField64;
use p3_fri::{FriBasedPcs, FriConfigImpl};
use p3_keccak::Keccak256Hash;
use p3_ldt::QuotientMmcs;
use p3_matrix::dense::RowMajorMatrix;
use p3_matrix::Matrix;
use p3_mds::coset_mds::CosetMds;
use p3_merkle_tree::FieldMerkleTreeMmcs;
use p3_poseidon2::{DiffusionMatrixBabybear, Poseidon2};
use p3_symmetric::{CompressionFunctionFromHasher, SerializingHasher32};
use p3_uni_stark::StarkConfigImpl;

use std::collections::BTreeMap;
use std::fmt::Debug;
mod builder;

pub use builder::InteractionBuilder;

use crate::runtime::Segment;

/// An interaction for a lookup or a permutation argument.
pub struct Interaction<F: Field> {
    pub values: Vec<VirtualPairCol<F>>,
    pub multiplicity: VirtualPairCol<F>,
    pub kind: InteractionKind,
}

// TODO: add debug for VirtualPairCol so that we can derive Debug for Interaction.
impl<F: Field> Debug for Interaction<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interaction")
            .field("kind", &self.kind)
            .finish()
    }
}

/// The type of interaction for a lookup argument.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractionKind {
    /// Interaction with the memory table, such as read and write.
    Memory = 1,
    /// Interaction with the program table, loading an instruction at a given pc address.
    Program = 2,
    /// Interaction with instruction oracle.
    Instruction = 3,
    /// Interaction with the ALU operations
    Alu = 4,
    /// Interaction with the byte lookup table for byte operations.
    Byte = 5,
    /// Requesting a range check for a given value and range.
    Range = 6,
    /// Interaction with the field op table for field operations.
    Field = 7,
}

impl<F: Field> Interaction<F> {
    pub fn new(
        values: Vec<VirtualPairCol<F>>,
        multiplicity: VirtualPairCol<F>,
        kind: InteractionKind,
    ) -> Self {
        Self {
            values,
            multiplicity,
            kind,
        }
    }

    pub fn argument_index(&self) -> usize {
        self.kind as usize
    }
}

#[derive(Debug)]
pub struct InteractionData<F: Field> {
    pub chip_name: String,
    pub kind: InteractionKind,
    pub row: usize,
    pub interaction_number: usize,
    pub is_send: bool,
    pub multiplicity: F,
}

pub fn vec_to_string<F: Field>(vec: Vec<F>) -> String {
    let mut result = String::from("(");
    for (i, value) in vec.iter().enumerate() {
        if i != 0 {
            result.push_str(", ");
        }
        result.push_str(&value.to_string());
    }
    result.push(')');
    result
}

fn babybear_to_int(n: BabyBear) -> i32 {
    let modulus = BabyBear::ORDER_U64;
    let val = n.as_canonical_u64();
    if val > modulus / 2 {
        val as i32 - modulus as i32
    } else {
        val as i32
    }
}

/// Calculate the the number of times we send and receive each event of the given interaction type,
/// and print out the ones for which the set of sends and receives don't match.
pub fn debug_interactions_with_all_chips(
    segment: &mut Segment,
    global_segment: Option<&mut Segment>,
    interaction_kinds: Vec<InteractionKind>,
) -> bool {
    if interaction_kinds.contains(&InteractionKind::Memory) && global_segment.is_none() {
        panic!("Memory interactions requires global segment.");
    }

    // Boilerplate code to set up the chips.
    type Val = BabyBear;
    type Domain = Val;
    type Challenge = BinomialExtensionField<Val, 4>;
    type PackedChallenge = BinomialExtensionField<<Domain as Field>::Packing, 4>;
    type MyMds = CosetMds<Val, 16>;
    type Perm = Poseidon2<Val, MyMds, DiffusionMatrixBabybear, 16, 5>;
    type MyHash = SerializingHasher32<Keccak256Hash>;
    type MyCompress = CompressionFunctionFromHasher<Val, MyHash, 2, 8>;
    type ValMmcs = FieldMerkleTreeMmcs<Val, MyHash, MyCompress, 8>;
    type ChallengeMmcs = ExtensionMmcs<Val, Challenge, ValMmcs>;
    type Dft = Radix2DitParallel;
    type Challenger = DuplexChallenger<Val, Perm, 16>;
    type Quotient = QuotientMmcs<Domain, Challenge, ValMmcs>;
    type MyFriConfig = FriConfigImpl<Val, Challenge, Quotient, ChallengeMmcs, Challenger>;
    type Pcs = FriBasedPcs<MyFriConfig, ValMmcs, Dft, Challenger>;
    type MyConfig = StarkConfigImpl<Val, Challenge, PackedChallenge, Pcs, Challenger>;

    // Here, we collect all the chips.
    let segment_chips = Runtime::segment_chips::<MyConfig>();
    let global_chips = Runtime::global_chips::<MyConfig>();

    let mut counts: Vec<(BTreeMap<String, BabyBear>, String)> = vec![];
    let mut final_map = BTreeMap::new();

    for chip in segment_chips {
        let (_, count) = debug_interactions::<BabyBear>(
            chip.as_ref(),
            &mut segment.clone(),
            interaction_kinds.clone(),
        );

        counts.push((count.clone(), chip.name()));
        tracing::debug!("{} chip has {} distinct events", chip.name(), count.len());
        for (key, value) in count.iter() {
            *final_map.entry(key.clone()).or_insert(0) += babybear_to_int(*value);
        }
    }

    if let Some(global_segment) = global_segment {
        for chip in global_chips {
            let (_, count) = debug_interactions::<BabyBear>(
                chip.as_ref(),
                &mut global_segment.clone(),
                interaction_kinds.clone(),
            );

            counts.push((count.clone(), chip.name()));
            tracing::debug!("{} chip has {} distinct events", chip.name(), count.len());
            for (key, value) in count.iter() {
                *final_map.entry(key.clone()).or_insert(0) += babybear_to_int(*value);
            }
        }
    }

    tracing::debug!("Final counts below.");
    tracing::debug!("==================");

    let mut any_nonzero = false;
    for (key, value) in final_map.clone() {
        if value != 0 {
            tracing::debug!(
                "Interaction key: {} Send-Receive Discrepancy: {}",
                key,
                value
            );
            any_nonzero = true;
            for count in counts.iter() {
                if count.0.contains_key(&key) {
                    tracing::debug!(
                        "{} chip's send-receive discrepancy for this key is {}",
                        count.1,
                        count.0[&key]
                    );
                }
            }
        }
    }

    tracing::debug!("==================");
    if !any_nonzero {
        tracing::debug!("All chips have the same number of sends and receives.");
    } else {
        tracing::debug!("Positive values mean sent more than received.");
        tracing::debug!("Negative values mean received more than sent.");
    }

    !any_nonzero
}

pub fn debug_interactions<F: Field>(
    chip: &dyn Chip<F>,
    segment: &mut Segment,
    interaction_kinds: Vec<InteractionKind>,
) -> (
    BTreeMap<String, Vec<InteractionData<F>>>,
    BTreeMap<String, F>,
) {
    let mut key_to_vec_data = BTreeMap::new();
    let mut key_to_count = BTreeMap::new();

    let trace: RowMajorMatrix<F> = chip.generate_trace(segment);
    let width = chip.width();
    let mut builder = InteractionBuilder::<F>::new(width);
    chip.eval(&mut builder);
    let mut main = trace.clone();
    let all_interactions = chip.all_interactions();
    let nb_send_interactions = chip.sends().len();
    let height = trace.clone().height();
    for row in 0..height {
        for (m, interaction) in all_interactions.iter().enumerate() {
            if !interaction_kinds.contains(&interaction.kind) {
                continue;
            }
            let is_send = m < nb_send_interactions;
            let multiplicity_eval = interaction
                .multiplicity
                .apply::<F, F>(&[], main.row_mut(row));

            if !multiplicity_eval.is_zero() {
                let mut values = vec![];
                for value in &interaction.values {
                    let expr = value.apply::<F, F>(&[], main.row_mut(row));
                    values.push(expr);
                }
                let key = vec_to_string(values);
                key_to_vec_data
                    .entry(key.clone())
                    .or_insert_with(Vec::new)
                    .push(InteractionData {
                        chip_name: chip.name(),
                        kind: interaction.kind,
                        row,
                        interaction_number: m,
                        is_send,
                        multiplicity: multiplicity_eval,
                    });
                let current = key_to_count.entry(key.clone()).or_insert(F::zero());
                if is_send {
                    *current += multiplicity_eval;
                } else {
                    *current -= multiplicity_eval;
                }
            }
        }
    }

    (key_to_vec_data, key_to_count)
}
