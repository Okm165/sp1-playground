mod fp;
mod fp2_addsub;
mod fp2_mul;

pub use fp::*;
pub use fp2_addsub::*;
pub use fp2_mul::*;

use crate::operations::field::params::{FieldParameters, NumWords};

#[derive(Debug)]
pub enum FieldType {
    Bls12381,
    Bn254,
}

pub trait FpOpField: FieldParameters + NumWords {
    const FIELD_TYPE: FieldType;
}

#[cfg(test)]
mod tests {
    use crate::utils::tests::{BLS12381_FP2_ADDSUB_ELF, BN254_FP_ELF};
    use crate::Program;
    use crate::{
        stark::CpuProver,
        utils::{
            self,
            tests::{BLS12381_FP2_MUL_ELF, BLS12381_FP_ELF},
        },
    };

    #[test]
    fn test_bls12381_fp() {
        utils::setup_logger();
        let program = Program::from(BLS12381_FP_ELF);
        utils::run_test::<CpuProver<_, _>>(program).unwrap();
    }

    #[test]
    fn test_bls12381_fp2_addsub() {
        utils::setup_logger();
        let program = Program::from(BLS12381_FP2_ADDSUB_ELF);
        utils::run_test::<CpuProver<_, _>>(program).unwrap();
    }

    #[test]
    fn test_bls12381_fp2_mul() {
        utils::setup_logger();
        let program = Program::from(BLS12381_FP2_MUL_ELF);
        utils::run_test::<CpuProver<_, _>>(program).unwrap();
    }

    #[test]
    fn test_bn254_fp() {
        utils::setup_logger();
        let program = Program::from(BN254_FP_ELF);
        utils::run_test::<CpuProver<_, _>>(program).unwrap();
    }
}
