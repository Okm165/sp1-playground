use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use crate::{
    ffi::{build_groth16, prove_groth16, test_groth16, verify_groth16},
    witness::GnarkWitness,
};

use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use sp1_recursion_compiler::{
    constraints::Constraint,
    ir::{Config, Witness},
};

/// A prover that can generate proofs with the Groth16 protocol using bindings to Gnark.
#[derive(Debug, Clone)]
pub struct Groth16Prover;

/// A zero-knowledge proof generated by the Groth16 protocol with a Base64 encoded gnark groth16 proof.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Groth16Proof {
    pub public_inputs: [String; 2],
    pub encoded_proof: String,
    pub raw_proof: String,
}

impl Groth16Prover {
    /// Creates a new [Groth16Prover].
    pub fn new() -> Self {
        Self
    }
    /// Executes the prover in testing mode with a circuit definition and witness.
    pub fn test<C: Config>(constraints: Vec<Constraint>, witness: Witness<C>) {
        let serialized = serde_json::to_string(&constraints).unwrap();

        // Write constraints.
        let mut constraints_file = tempfile::NamedTempFile::new().unwrap();
        constraints_file.write_all(serialized.as_bytes()).unwrap();

        // Write witness.
        let mut witness_file = tempfile::NamedTempFile::new().unwrap();
        let gnark_witness = GnarkWitness::new(witness);
        let serialized = serde_json::to_string(&gnark_witness).unwrap();
        witness_file.write_all(serialized.as_bytes()).unwrap();

        std::env::set_var("WITNESS_JSON", witness_file.path().to_str().unwrap());
        std::env::set_var(
            "CONSTRAINTS_JSON",
            constraints_file.path().to_str().unwrap(),
        );
        test_groth16();
    }

    pub fn build<C: Config>(constraints: Vec<Constraint>, witness: Witness<C>, build_dir: PathBuf) {
        let serialized = serde_json::to_string(&constraints).unwrap();

        // Write constraints.
        let constraints_path = build_dir.join("constraints_groth16.json");
        let mut file = File::create(constraints_path).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();

        // Write witness.
        let witness_path = build_dir.join("witness_groth16.json");
        let gnark_witness = GnarkWitness::new(witness);
        let mut file = File::create(witness_path).unwrap();
        let serialized = serde_json::to_string(&gnark_witness).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();

        build_groth16(build_dir.to_str().unwrap());
    }

    /// Generates a Groth16 proof by sending a request to the Gnark server.
    pub fn prove<C: Config>(&self, witness: Witness<C>, build_dir: PathBuf) -> Groth16Proof {
        // Write witness.
        let mut witness_file = tempfile::NamedTempFile::new().unwrap();
        let gnark_witness = GnarkWitness::new(witness);
        let serialized = serde_json::to_string(&gnark_witness).unwrap();
        witness_file.write_all(serialized.as_bytes()).unwrap();

        prove_groth16(
            build_dir.to_str().unwrap(),
            witness_file.path().to_str().unwrap(),
        )
    }

    pub fn verify(
        &self,
        proof: &Groth16Proof,
        vkey_hash: &BigUint,
        commited_values_digest: &BigUint,
        build_dir: &Path,
    ) {
        verify_groth16(
            build_dir.to_str().unwrap(),
            &proof.raw_proof,
            &vkey_hash.to_string(),
            &commited_values_digest.to_string(),
        )
        .expect("failed to verify proof")
    }
}

impl Default for Groth16Prover {
    fn default() -> Self {
        Self::new()
    }
}
