use succinct_core::{utils, SuccinctProver};

const FIBONACCI_ELF: &[u8] =
    include_bytes!("../../../programs/demo/fibonacci/elf/riscv32im-succinct-zkvm-elf");

fn main() {
    std::env::set_var("RUST_LOG", "info");
    utils::setup_logger();
    let prover = SuccinctProver::new();
    prover.run_and_prove(FIBONACCI_ELF);
}
