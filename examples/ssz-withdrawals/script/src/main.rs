use sp1_sdk::{utils, ProverClient, SP1ProofWithPublicValues, SP1Stdin};

const ELF: &[u8] = include_bytes!("../../program/elf/riscv32im-succinct-zkvm-elf");

fn main() {
    // Generate proof.
    // utils::setup_tracer();
    utils::setup_logger();

    let stdin = SP1Stdin::new();
    let client = ProverClient::new();
    let (pk, vk) = client.setup(ELF);

    let executor = client
        .execute(ELF, stdin.clone())
        .run()
        .expect("proving failed");

    // let proof = client.prove(&pk, stdin).run().expect("proving failed");

    // // Verify proof.
    // client.verify(&proof, &vk).expect("verification failed");

    // // Test a round trip of proof serialization and deserialization.
    // proof
    //     .save("proof-with-pis.bin")
    //     .expect("saving proof failed");
    // let deserialized_proof =
    //     SP1ProofWithPublicValues::load("proof-with-pis.bin").expect("loading proof failed");

    // // Verify the deserialized proof.
    // client
    //     .verify(&deserialized_proof, &vk)
    //     .expect("verification failed");

    // println!("successfully generated and verified proof for the program!")
}
