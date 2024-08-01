#[rustfmt::skip]
pub mod proto {
    pub mod api;
}

use serde::{Deserialize, Serialize};
use sp1_core::io::SP1Stdin;
use sp1_core::stark::ShardProof;
use sp1_core::utils::SP1ProverOpts;
use sp1_prover::types::SP1ProvingKey;
use sp1_prover::InnerSC;
use sp1_prover::SP1CoreProof;
use sp1_prover::SP1VerifyingKey;

#[derive(Serialize, Deserialize)]
pub struct ProveCoreRequestPayload {
    pub pk: SP1ProvingKey,
    pub stdin: SP1Stdin,
}

#[derive(Serialize, Deserialize)]
pub struct CompressRequestPayload {
    pub vk: SP1VerifyingKey,
    pub proof: SP1CoreProof,
    pub deferred_proofs: Vec<ShardProof<InnerSC>>,
}

#[cfg(test)]
mod tests {
    use sp1_core::utils::tests::FIBONACCI_ELF;
    use sp1_prover::components::DefaultProverComponents;
    use sp1_prover::{InnerSC, SP1CoreProof, SP1Prover, SP1ReduceProof};
    use twirp::url::Url;
    use twirp::Client;

    use crate::CompressRequestPayload;
    use crate::SP1Stdin;
    use crate::{proto::api::ProverServiceClient, ProveCoreRequestPayload};

    #[tokio::test]
    async fn test_prove_core() {
        let client =
            Client::from_base_url(Url::parse("http://localhost:3000/twirp/").unwrap()).unwrap();

        let prover = SP1Prover::<DefaultProverComponents>::new();
        let (pk, vk) = prover.setup(FIBONACCI_ELF);
        let payload = ProveCoreRequestPayload {
            pk,
            stdin: SP1Stdin::new(),
        };
        let request = crate::proto::api::ProveCoreRequest {
            data: bincode::serialize(&payload).unwrap(),
        };
        let proof = client.prove_core(request).await.unwrap();
        let proof: SP1CoreProof = bincode::deserialize(&proof.result).unwrap();
        prover.verify(&proof.proof, &vk).unwrap();

        tracing::info!("compress");
        let payload = CompressRequestPayload {
            vk: vk.clone(),
            proof,
            deferred_proofs: vec![],
        };
        let request = crate::proto::api::CompressRequest {
            data: bincode::serialize(&payload).unwrap(),
        };
        let compressed_proof = client.compress(request).await.unwrap();
        let compressed_proof: SP1ReduceProof<InnerSC> =
            bincode::deserialize(&compressed_proof.result).unwrap();

        tracing::info!("verify compressed");
        prover.verify_compressed(&compressed_proof, &vk).unwrap();
    }
}
