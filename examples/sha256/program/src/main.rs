#![no_main]
sp1_zkvm::entrypoint!(main);

use sha2::{Digest, Sha256};

pub fn main() {
    let input = sp1_zkvm::io::read::<Vec<u8>>();
    let hash = Sha256::digest(&input);
    let mut ret = [0u8; 32];
    ret.copy_from_slice(&hash);
    println!("{:?}", ret);
}
