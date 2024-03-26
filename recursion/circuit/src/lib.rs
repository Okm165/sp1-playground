use p3_baby_bear::BabyBear;
use p3_bn254_fr::Bn254Fr;
use p3_field::extension::BinomialExtensionField;
use sp1_recursion_compiler::ir::Config;

pub mod challenger;
pub mod poseidon2;

#[derive(Clone)]
pub struct GnarkConfig;

impl Config for GnarkConfig {
    type N = Bn254Fr;
    type F = BabyBear;
    type EF = BinomialExtensionField<BabyBear, 4>;
}
