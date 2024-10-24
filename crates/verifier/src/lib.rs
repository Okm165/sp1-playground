//! This crate provides verifiers for SP1 Groth16 and Plonk BN254 proofs in a no-std environment.
//! It is patched for efficient verification within the SP1 ZKVM context.
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![no_std]
extern crate alloc;

use lazy_static::lazy_static;

lazy_static! {
    /// The PLONK verifying key for this SP1 version.
    pub static ref PLONK_VK_BYTES: &'static [u8] = include_bytes!("../bn254-vk/plonk_vk.bin");
}

lazy_static! {
    /// The Groth16 verifying key for this SP1 version.
    pub static ref GROTH16_VK_BYTES: &'static [u8] = include_bytes!("../bn254-vk/groth16_vk.bin");
}

mod constants;
mod converter;
mod error;
mod groth16;
mod utils;

pub use groth16::Groth16Verifier;
pub use utils::*;

#[cfg(feature = "getrandom")]
mod plonk;
#[cfg(feature = "getrandom")]
pub use plonk::PlonkVerifier;

#[cfg(test)]
mod tests;