#![no_main]
sp1_zkvm::entrypoint!(main);
use sp1_zkvm::syscalls::syscall_poseidon2_permute;

pub fn main() {
    for _ in 0..25 {
        let input: [u32; 16] = (0..16).collect::<Vec<u32>>().try_into().unwrap();
        let mut output = [0; 16];
        syscall_poseidon2_permute(&input, &mut output);
        println!("{:?}", output);
        assert_eq!(
            output,
            // Output generated with https://github.com/HorizenLabs/poseidon2 POSEIDON2_BABYBEAR_16_PARAMS
            [
                896560466, 771677727, 128113032, 1378976435, 160019712, 1452738514, 682850273,
                223500421, 501450187, 1804685789, 1671399593, 1788755219, 1736880027, 1352180784,
                1928489698, 1128802977
            ]
        );
    }
}
