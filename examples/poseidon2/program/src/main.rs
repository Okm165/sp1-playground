#![no_main]

use sp1_zkvm::syscalls::syscall_poseidon2_permute;

sp1_zkvm::entrypoint!(main);

pub fn main() {
    let mut state = [10_u32; 16];
    let mut permuted = [0_u32; 16];
    syscall_poseidon2_permute(&mut state, &mut permuted);
    println!("{:?}", state);
    println!("{:?}", permuted);
}
