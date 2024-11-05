//! A program that takes a number `n` as input, and writes if `n` is prime as an output.
use sp1_sdk::{include_elf, utils, ProverClient, SP1Stdin};

const ELF: &[u8] = include_elf!("sha256-program");

fn main() {
    // Setup a tracer for logging.
    utils::setup_logger();

    let mut stdin = SP1Stdin::new();

    // // Create an input stream and write '29' to it
    // let n = 29u64;
    stdin.write(&include_bytes!("../../../myfile").to_vec());

    // Generate and verify the proof
    let client = ProverClient::new();
    let (pk, vk) = client.setup(ELF);

    // let (output, report) = client.execute(ELF, stdin).run().unwrap();
    // println!("{}", report);

    let mut proof = client.prove(&pk, stdin).run().unwrap();

    // // let is_prime = proof.public_values.read::<bool>();
    // // println!("Is 29 prime? {}", is_prime);

    // client.verify(&proof, &vk).expect("verification failed");

    // // Test a round trip of proof serialization and deserialization.
    // proof.save("proof-with-sha256.bin").expect("saving proof failed");
    // let deserialized_proof =
    //     SP1ProofWithPublicValues::load("proof-with-sha256.bin").expect("loading proof failed");

    // // Verify the deserialized proof.
    // client.verify(&deserialized_proof, &vk).expect("verification failed");

    // println!("successfully generated and verified proof for the program!")
}
