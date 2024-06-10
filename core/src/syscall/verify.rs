use core::panic;

use crate::runtime::{Syscall, SyscallContext};

/// Verifies an SP1 recursive verifier proof. Note that this syscall only verifies the proof during
/// runtime. The actual constraint-level verification is deferred to the recursive layer, where
/// proofs are witnessed and verified in order to reconstruct the deferred_proofs_digest.
pub struct SyscallVerifySP1Proof;

impl SyscallVerifySP1Proof {
    pub const fn new() -> Self {
        Self
    }
}

impl Syscall for SyscallVerifySP1Proof {
    fn execute(&self, ctx: &mut SyscallContext, vkey_ptr: u32, pv_digest_ptr: u32) -> Option<u32> {
        let rt = &mut ctx.rt;

        // vkey_ptr is a pointer to [u32; 8] which contains the verification key.
        assert_eq!(vkey_ptr % 4, 0, "vkey_ptr must be word-aligned");
        // pv_digest_ptr is a pointer to [u32; 8] which contains the public values digest.
        assert_eq!(pv_digest_ptr % 4, 0, "pv_digest_ptr must be word-aligned");

        let vkey = (0..8)
            .map(|i| rt.word(vkey_ptr + i * 4))
            .collect::<Vec<u32>>();

        let pv_digest = (0..8)
            .map(|i| rt.word(pv_digest_ptr + i * 4))
            .collect::<Vec<u32>>();

        let proof_index = rt.state.proof_stream_ptr;
        if proof_index >= rt.state.proof_stream.len() {
            panic!("Not enough proofs were written to the runtime.");
        }
        let (proof, proof_vk) = &rt.state.proof_stream[proof_index].clone();
        rt.state.proof_stream_ptr += 1;

        let vkey_bytes: [u32; 8] = vkey.try_into().unwrap();
        let pv_digest_bytes: [u32; 8] = pv_digest.try_into().unwrap();

        ctx.rt
            .deferred_proof_verifier
            .verify_deferred_proof(proof, proof_vk, vkey_bytes, pv_digest_bytes)
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to verify proof {proof_index} with digest {}: {}",
                    hex::encode(bytemuck::cast_slice(&pv_digest_bytes)),
                    e
                )
            });

        None
    }
}
