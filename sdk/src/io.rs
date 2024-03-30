use serde::{de::DeserializeOwned, Deserialize, Serialize};

use sp1_core::stark::{Proof, StarkGenericConfig};
use sp1_core::utils::Buffer;

/// Standard input for the prover.
#[derive(Serialize, Deserialize)]
pub struct SP1Stdin {
    pub buffer: Buffer,
}

/// Standard output for the prover.
#[derive(Serialize, Deserialize)]
pub struct SP1PublicValues {
    pub buffer: Buffer,
}

impl Default for SP1Stdin {
    fn default() -> Self {
        Self::new()
    }
}

impl SP1Stdin {
    /// Create a new `SP1Stdin`.
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(),
        }
    }

    /// Create a `SP1Stdin` from a slice of bytes.
    pub fn from(data: &[u8]) -> Self {
        Self {
            buffer: Buffer::from(data),
        }
    }

    /// Read a value from the buffer.
    pub fn read<T: Serialize + DeserializeOwned>(&mut self) -> T {
        self.buffer.read()
    }

    /// Read a slice of bytes from the buffer.
    pub fn read_slice(&mut self, slice: &mut [u8]) {
        self.buffer.read_slice(slice);
    }

    /// Write a value to the buffer.
    pub fn write<T: Serialize>(&mut self, data: &T) {
        self.buffer.write(data);
    }

    /// Write a slice of bytes to the buffer.
    pub fn write_slice(&mut self, slice: &[u8]) {
        self.buffer.write_slice(slice);
    }
}

impl Default for SP1PublicValues {
    fn default() -> Self {
        Self::new()
    }
}

impl SP1PublicValues {
    /// Create a new `SP1PublicValues`.
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(),
        }
    }

    /// Create a `SP1PublicValues` from a slice of bytes.
    pub fn from(data: &[u8]) -> Self {
        Self {
            buffer: Buffer::from(data),
        }
    }

    /// Read a value from the buffer.    
    pub fn read<T: Serialize + DeserializeOwned>(&mut self) -> T {
        self.buffer.read()
    }

    /// Read a slice of bytes from the buffer.
    pub fn read_slice(&mut self, slice: &mut [u8]) {
        self.buffer.read_slice(slice);
    }

    /// Write a value to the buffer.
    pub fn write<T: Serialize + DeserializeOwned>(&mut self, data: &T) {
        self.buffer.write(data);
    }

    /// Write a slice of bytes to the buffer.
    pub fn write_slice(&mut self, slice: &[u8]) {
        self.buffer.write_slice(slice);
    }
}

pub mod proof_serde {
    use super::*;
    use rmp_serde::{decode as rmp_decode, encode as rmp_encode};
    use serde::{Deserializer, Serializer};

    pub fn serialize<S, SC: StarkGenericConfig + Serialize>(
        proof: &Proof<SC>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let mut bytes = Vec::new();
            rmp_encode::write_named(&mut bytes, proof).unwrap();
            let hex_bytes = hex::encode(bytes);
            serializer.serialize_str(&hex_bytes)
        } else {
            proof.serialize(serializer)
        }
    }

    pub fn deserialize<'de, D, SC: StarkGenericConfig + DeserializeOwned>(
        deserializer: D,
    ) -> Result<Proof<SC>, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let hex_bytes = String::deserialize(deserializer).unwrap();
            let bytes = hex::decode(hex_bytes).unwrap();
            let mut de = rmp_decode::Deserializer::new(&bytes[..]);
            Proof::<SC>::deserialize(&mut de).map_err(serde::de::Error::custom)
        } else {
            Proof::<SC>::deserialize(deserializer)
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::{SP1ProofWithIO, SP1Prover, SP1Stdin, SP1Verifier};

        pub const FIBONACCI_IO_ELF: &[u8] =
            include_bytes!("../../examples/fibonacci-io/program/elf/riscv32im-succinct-zkvm-elf");

        /// Tests serialization with a human-readable encoding (JSON)
        #[test]
        fn test_json_roundtrip() {
            let mut stdin = SP1Stdin::new();
            stdin.write(&3u32);
            let proof = SP1Prover::prove(FIBONACCI_IO_ELF, stdin).unwrap();
            let json = serde_json::to_string(&proof).unwrap();
            let output = serde_json::from_str::<SP1ProofWithIO<_>>(&json).unwrap();
            SP1Verifier::verify(FIBONACCI_IO_ELF, &output).unwrap();
        }

        /// Tests serialization with MsgPack encoding
        #[test]
        fn test_msgpack_roundtrip() {
            let mut stdin = SP1Stdin::new();
            stdin.write(&3u32);
            let proof = SP1Prover::prove(FIBONACCI_IO_ELF, stdin).unwrap();
            let serialized = rmp_serde::to_vec(&proof).expect("Failed to serialize with MsgPack");
            let output: SP1ProofWithIO<_> =
                rmp_serde::from_slice(&serialized).expect("Failed to deserialize with MsgPack");

            SP1Verifier::verify(FIBONACCI_IO_ELF, &output).unwrap();
        }
    }
}
