#![no_main]

use p3_baby_bear::BabyBear;
use p3_field::AbstractField;
use p3_symmetric::CryptographicHasher;
use sp1_lib::poseidon2::Poseidon2;
use sp1_primitives::poseidon2_hasher;
sp1_zkvm::entrypoint!(main);

pub fn main() {
    let input = (0..10000).map(BabyBear::from_canonical_u32).collect::<Vec<BabyBear>>();

    println!("cycle-tracker-report-start: precompile");
    let output_precompile = Poseidon2::<8, 8>::hash_iter(input.iter());
    println!("cycle-tracker-report-end: precompile");

    println!("cycle-tracker-report-start: native");
    let output_native = poseidon2_hasher().hash_iter(input);
    println!("cycle-tracker-report-end: native");

    assert_eq!(output_precompile, output_native);
}
