use crate::{
    stark::{ShardProof, StarkVerifyingKey},
    utils::{BabyBearPoseidon2, Buffer},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{convert::TryInto, io::Read};

// Serialize and align data to u32 boundaries
fn serialize_to_u32_aligned<T: Serialize>(data: &T) -> Vec<u8> {
    let serialized = bincode::serialize(data).expect("Failed to serialize data");
    let padding_size = (4 - serialized.len() % 4) % 4;
    let mut aligned_data = serialized;
    aligned_data.resize(aligned_data.len() + padding_size, 0); // Add padding
    aligned_data
}

// Deserialize data assuming it is u32 aligned
fn deserialize_from_u32_aligned<T: DeserializeOwned>(data: &[u8]) -> T {
    let actual_length = data.len() - (data.len() % 4);
    let aligned_data = &data[..actual_length];
    bincode::deserialize(aligned_data).expect("Failed to deserialize data")
}

/// Standard input for the prover.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SP1Stdin {
    /// Input stored as a vec of vec of bytes. It's stored this way because the read syscall reads
    /// a vec of bytes at a time.
    pub buffer: Vec<Vec<u8>>,
    pub ptr: usize,
    pub proofs: Vec<(
        ShardProof<BabyBearPoseidon2>,
        StarkVerifyingKey<BabyBearPoseidon2>,
    )>,
}

/// Public values for the prover.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SP1PublicValues {
    buffer: Buffer,
}

impl SP1Stdin {
    /// Create a new `SP1Stdin`.
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            ptr: 0,
            proofs: Vec::new(),
        }
    }

    /// Create a `SP1Stdin` from a slice of bytes.
    pub fn from(data: &[u8]) -> Self {
        Self {
            buffer: vec![data.to_vec()],
            ptr: 0,
            proofs: Vec::new(),
        }
    }

    /// Read a value from the buffer.
    pub fn read<T: Serialize + DeserializeOwned>(&mut self) -> T {
        let data = &self.buffer[self.ptr];
        self.ptr += 1; // Increment the pointer after reading
        deserialize_from_u32_aligned(data)
    }

    /// Read a slice of bytes from the buffer.
    pub fn read_slice(&mut self, slice: &mut [u8]) {
        slice.copy_from_slice(&self.buffer[self.ptr]);
        self.ptr += 1;
    }

    /// Write a value to the buffer.
    pub fn write<T: Serialize>(&mut self, data: &T) {
        let mut tmp = Vec::new();
        bincode::serialize_into(&mut tmp, data).expect("serialization failed");
        self.buffer.push(tmp);
    }

    /// Write a slice of bytes to the buffer.
    pub fn write_slice(&mut self, slice: &[u8]) {
        self.buffer.push(slice.to_vec());
    }

    pub fn write_vec(&mut self, vec: Vec<u8>) {
        self.buffer.push(vec);
    }

    pub fn write_proof(
        &mut self,
        proof: ShardProof<BabyBearPoseidon2>,
        vk: StarkVerifyingKey<BabyBearPoseidon2>,
    ) {
        self.proofs.push((proof, vk));
    }
}

impl SP1PublicValues {
    /// Create a new `SP1PublicValues`.
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(),
        }
    }

    pub fn bytes(&self) -> String {
        format!("0x{}", hex::encode(self.buffer.data.clone()))
    }

    /// Create a `SP1PublicValues` from a slice of bytes.
    pub fn from(data: &[u8]) -> Self {
        Self {
            buffer: Buffer::from(data),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.buffer.data.as_slice()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.buffer.data.clone()
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

impl AsRef<[u8]> for SP1PublicValues {
    fn as_ref(&self) -> &[u8] {
        &self.buffer.data
    }
}

pub mod proof_serde {
    use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};

    use crate::stark::{MachineProof, StarkGenericConfig};

    pub fn serialize<S, SC: StarkGenericConfig + Serialize>(
        proof: &MachineProof<SC>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            let bytes = bincode::serialize(proof).unwrap();
            let hex_bytes = hex::encode(bytes);
            serializer.serialize_str(&hex_bytes)
        } else {
            proof.serialize(serializer)
        }
    }

    pub fn deserialize<'de, D, SC: StarkGenericConfig + DeserializeOwned>(
        deserializer: D,
    ) -> Result<MachineProof<SC>, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let hex_bytes = String::deserialize(deserializer).unwrap();
            let bytes = hex::decode(hex_bytes).unwrap();
            let proof = bincode::deserialize(&bytes).map_err(serde::de::Error::custom)?;
            Ok(proof)
        } else {
            MachineProof::<SC>::deserialize(deserializer)
        }
    }
}
