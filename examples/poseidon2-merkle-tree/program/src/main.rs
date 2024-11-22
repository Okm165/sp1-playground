#![no_main]

use p3_baby_bear::BabyBear;
use p3_field::AbstractField;
use sp1_lib::poseidon_hash::Poseidon2;

sp1_zkvm::entrypoint!(main);

#[derive(Debug, Clone)]
pub struct MerkleTree {
    leaves: Vec<BabyBear>,
    nodes: Vec<BabyBear>, // Flattened array of tree nodes
}

impl MerkleTree {
    /// Constructs a new Merkle tree from the given leaves
    pub fn new(leaves: Vec<BabyBear>) -> Self {
        assert!(!leaves.is_empty(), "Merkle tree cannot be empty.");

        let mut nodes = leaves.clone(); // Start with leaves
        let mut current_level = leaves.clone();

        // Build tree from leaves to root
        while current_level.len() > 1 {
            let mut next_level = vec![];
            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    BabyBear::zero()
                };
                let parent = Poseidon2::hash_two(left, right);
                next_level.push(parent);
            }
            nodes.extend(&next_level); // Append the next level to the flattened tree
            current_level = next_level;
        }

        MerkleTree { leaves, nodes }
    }

    /// Returns the Merkle tree root
    pub fn root(&self) -> BabyBear {
        self.nodes.last().copied().unwrap_or_else(BabyBear::zero)
    }

    /// Generates a proof for a given leaf index
    pub fn generate_proof(&self, index: usize) -> Vec<(BabyBear, bool)> {
        assert!(index < self.leaves.len(), "Index out of range.");

        let mut proof = vec![];
        let mut current_index = index;

        let mut level_start = 0;
        let mut level_size = self.leaves.len();

        // Traverse tree levels upwards
        while level_size > 1 {
            let is_left = current_index % 2 == 0;
            let sibling_index = if is_left { current_index + 1 } else { current_index - 1 };

            if sibling_index < level_size {
                proof.push((self.nodes[level_start + sibling_index], !is_left));
            }

            current_index /= 2; // Move to parent index
            level_start += level_size;
            level_size = (level_size + 1) / 2; // Compute next level size
        }

        proof
    }

    /// Verifies a Merkle proof against the provided root
    pub fn verify_proof(root: BabyBear, leaf: BabyBear, proof: Vec<(BabyBear, bool)>) -> bool {
        let mut hash = leaf;
        for (sibling, is_right) in proof {
            hash = if is_right {
                Poseidon2::hash_two(sibling, hash)
            } else {
                Poseidon2::hash_two(hash, sibling)
            };
        }
        hash == root
    }
}

pub fn main() {
    let leaves = (0..100).map(BabyBear::from_canonical_u32).collect();
    let merkle_tree = MerkleTree::new(leaves);

    let index: usize = 67;

    assert!(MerkleTree::verify_proof(
        merkle_tree.root(),
        BabyBear::from_canonical_usize(index),
        merkle_tree.generate_proof(index)
    ));
}
