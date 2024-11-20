#![no_main]

use sp1_zkvm::syscalls::syscall_poseidon2_permute;

sp1_zkvm::entrypoint!(main);

pub fn main() {
    let input = [10_u32; 16];
    let mut output = [0_u32; 16];
    syscall_poseidon2_permute(&input, &mut output);
    println!("{:?}", output);
}
