#![no_main]

use p3_baby_bear::BabyBear;
use p3_field::AbstractField;
use sp1_lib::poseidon_hash::Poseidon2;

sp1_zkvm::entrypoint!(main);

pub fn main() {
    let hash = Poseidon2::hash_two(BabyBear::zero(), BabyBear::one());
    println!("{:?}", hash);
}
