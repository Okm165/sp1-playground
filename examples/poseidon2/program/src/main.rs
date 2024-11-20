#![no_main]

use sp1_zkvm::syscalls::syscall_poseidon2_permute;

sp1_zkvm::entrypoint!(main);

pub fn main() {
    let input = [1_u32; 16];
    let mut output = [0_u32; 16];
    syscall_poseidon2_permute(&input, &mut output);
    assert_eq!([617459174, 301624730, 256512781, 680281790, 644113761, 1800568483, 149729887, 1932613063, 704444624, 1254517431, 1741614496, 43875254, 425567108, 1908246830, 1184691966, 1655842814], output);
}
