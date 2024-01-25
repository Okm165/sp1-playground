#![no_main]

extern crate succinct_zkvm;
use hex_literal::hex;
use sha2::{Digest, Sha256};

succinct_zkvm::entrypoint!(main);

pub fn main() {
    println!("cycle-tracker-start: sha256");
    let hash = Sha256::digest(b"hello world");
    println!("cycle-tracker-end: sha256");
    let mut ret = [0u8; 32];
    ret.copy_from_slice(&hash);
    println!("{}", hex::encode(ret));
    assert_eq!(
        ret,
        hex!("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9")
    );
}
