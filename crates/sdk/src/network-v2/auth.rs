use std::{borrow::Cow, str::FromStr};

use alloy_sol_types::{sol, Eip712Domain, SolStruct};
use anyhow::Result;
use ethers::{
    signers::{LocalWallet, Signer},
    types::H256,
};

sol! {
    struct CreateProof {
        uint64 nonce;
        uint64 deadline;
        uint32 mode;
        string version;
    }
}

/// Handles authentication for the Succinct prover network. All interactions that could potentially
/// use computational resources must be authenticated by signing a message with a secp256k1 key.
///
/// The messages themselves follow EIP-712, where the domain is "succinct" and the TypeStruct
/// changes depending on which endpoint is being used. Documentation for EIP-712 can be found at:
/// https://eips.ethereum.org/EIPS/eip-712
pub struct NetworkAuth {
    // Holds a secp256k1 private key.
    wallet: LocalWallet,
}

impl NetworkAuth {
    pub fn new(private_key: &str) -> Self {
        let wallet = LocalWallet::from_str(private_key).unwrap();
        Self { wallet }
    }

    /// Gets the EIP-712 domain separator for the Succinct prover network.
    fn get_domain_separator() -> Eip712Domain {
        Eip712Domain {
            name: Some(Cow::Borrowed("succinct")),
            version: Some(Cow::Borrowed("1")),
            ..Default::default()
        }
    }

    /// Gets the address of the auth's account, derived from the secp256k1 private key.
    pub fn get_address(&self) -> [u8; 20] {
        self.wallet.address().0
    }

    // Generic function to sign a message based on the SolStruct.
    async fn sign_message<T: SolStruct>(&self, type_struct: T) -> Result<Vec<u8>> {
        let domain_separator = Self::get_domain_separator();
        let message_hash = type_struct.eip712_signing_hash(&domain_separator);
        let signature = self.wallet.sign_hash(H256(message_hash.0))?;
        Ok(signature.to_vec())
    }

    // pub async fn sign_create_proof_message(
    //     &self,
    //     nonce: u64,
    //     deadline: u64,
    //     mode: i32,
    //     version: &str,
    // ) -> Result<Vec<u8>> {
    //     let type_struct =
    //         CreateProof { nonce, deadline, mode: mode as u32, version: version.to_string() };
    //     self.sign_message(type_struct).await
    // }
}
