use sp1_sdk::{include_elf, ProverClient, SP1Stdin};
pub const ELF: &[u8] = include_elf!("poseidon2-program");

fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();

    let stdin = SP1Stdin::new();

    let client = ProverClient::new();

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    let (_, report) = client.execute(ELF, stdin.clone()).run().unwrap();
    println!("executed program with {} cycles", report.total_instruction_count());
    println!("precompile based Poseidon2: {:?} cycles", report.cycle_tracker.get("precompile").unwrap());
    println!("native based Poseidon2: {:?} cycles", report.cycle_tracker.get("native").unwrap());
}
