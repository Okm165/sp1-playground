#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
pub mod proto {
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub mod network;
}
pub mod client;
mod io;
mod util;
pub mod utils {
    pub use sp1_core::utils::{
        setup_logger, setup_tracer, BabyBearBlake3, BabyBearKeccak, BabyBearPoseidon2,
    };
}
pub use crate::io::*;
use proto::network::{ProofStatus, TransactionStatus};
use utils::*;

use crate::client::NetworkClient;
use anyhow::{Context, Ok, Result};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sp1_core::runtime::{Program, Runtime};
use sp1_core::stark::{Com, PcsProverData, RiscvAir};
use sp1_core::stark::{
    OpeningProof, ProgramVerificationError, Proof, ShardMainData, StarkGenericConfig,
};
use sp1_core::utils::run_and_prove;
use std::env;
use std::fs;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use util::StageProgressBar;

/// A prover that can prove RISCV ELFs.
pub struct SP1Prover;

/// A verifier that can verify proofs generated by `SP1Prover`.
pub struct SP1Verifier;

/// A proof of a RISCV ELF execution with given inputs and outputs.
#[derive(Serialize, Deserialize)]
pub struct SP1ProofWithIO<SC: StarkGenericConfig + Serialize + DeserializeOwned> {
    #[serde(with = "proof_serde")]
    pub proof: Proof<SC>,
    pub stdin: SP1Stdin,
    pub public_values: SP1PublicValues,
}

impl SP1Prover {
    /// Executes the elf with the given inputs and returns the output.
    pub fn execute(elf: &[u8], stdin: SP1Stdin) -> Result<SP1PublicValues> {
        let program = Program::from(elf);
        let mut runtime = Runtime::new(program);
        runtime.write_vecs(&stdin.buffer);
        runtime.run();
        Ok(SP1PublicValues::from(&runtime.state.public_values_stream))
    }

    /// Generate a proof for the execution of the ELF with the given public inputs.
    pub fn prove(elf: &[u8], stdin: SP1Stdin) -> Result<SP1ProofWithIO<BabyBearPoseidon2>> {
        Self::prove_with_config(elf, stdin, BabyBearPoseidon2::new())
    }

    async fn prove_remote<SC: StarkGenericConfig>(
        elf: &[u8],
        stdin: SP1Stdin,
    ) -> Result<(SP1ProofWithIO<SC>, Option<String>)>
    where
        SC: StarkGenericConfig,
        SC::Challenger: Clone,
        OpeningProof<SC>: Send + Sync,
        Com<SC>: Send + Sync,
        PcsProverData<SC>: Send + Sync,
        ShardMainData<SC>: Serialize + DeserializeOwned,
        SC::Val: p3_field::PrimeField32,
    {
        let access_token = std::env::var("PROVER_NETWORK_ACCESS_TOKEN").unwrap();
        let client = NetworkClient::with_token(access_token);
        let flat_stdin = stdin
            .buffer
            .iter()
            .flat_map(|v| v.iter())
            .copied()
            .collect::<Vec<u8>>();
        let id = client.create_proof(elf, &flat_stdin).await?;

        let mut pb = StageProgressBar::new();
        loop {
            let status = client.get_proof_status(&id).await;
            match status {
                std::result::Result::Ok(status) => {
                    if status.0.status() == ProofStatus::ProofFailed {
                        pb.finish();
                        return Err(anyhow::anyhow!("Proof failed"));
                    }
                    if let Some(result) = status.1 {
                        println!("Proof succeeded\n\n");
                        pb.finish();
                        return Ok((result, Some(id)));
                    }
                    pb.update(
                        status.0.stage,
                        status.0.total_stages,
                        &status.0.stage_name,
                        status.0.stage_progress.map(|p| (p, status.0.stage_total())),
                    );
                }
                Err(e) => {
                    pb.finish();
                    return Err(e);
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
    }

    pub async fn relay_remote(
        access_token: String,
        proof_id: &str,
        chain_ids: Vec<u32>,
        callbacks: Vec<&str>,
        callback_datas: Vec<&str>,
    ) -> Result<Vec<String>> {
        let client = NetworkClient::with_token(access_token);
        let verifier = &NetworkClient::get_sp1_verifier_address();

        let mut tx_details = Vec::new();
        for ((i, &callback), &callback_data) in
            callbacks.iter().enumerate().zip(callback_datas.iter())
        {
            if let Some(&chain_id) = chain_ids.get(i) {
                let tx_id = client
                    .relay_proof(proof_id, chain_id, verifier, callback, callback_data)
                    .await
                    .with_context(|| format!("Failed to relay proof to chain {}", chain_id))?;
                tx_details.push((tx_id, chain_id));
            }
        }

        for (tx_id, chain_id) in tx_details.iter() {
            loop {
                let (status_res, maybe_tx_hash, maybe_simulation_url) =
                    client.get_relay_status(tx_id).await?;

                match status_res.status() {
                    TransactionStatus::TransactionFinalized => {
                        println!(
                            "Relaying to chain {} succeeded with tx hash: {:?}",
                            chain_id,
                            maybe_tx_hash.unwrap_or("None".to_string())
                        );
                        break;
                    }
                    TransactionStatus::TransactionFailed
                    | TransactionStatus::TransactionTimedout => {
                        return Err(anyhow::anyhow!(
                            "Relaying to chain {} failed with tx hash: {:?}, simulation url: {:?}",
                            chain_id,
                            maybe_tx_hash.unwrap_or("None".to_string()),
                            maybe_simulation_url.unwrap_or("None".to_string())
                        ));
                    }
                    _ => {
                        std::thread::sleep(Duration::from_secs(5));
                    }
                }
            }
        }

        Ok(tx_details.into_iter().map(|(tx_id, _)| tx_id).collect())
    }

    pub async fn relay_proof_if_required(proof_id: String) -> Result<()> {
        if let std::result::Result::Ok(chains_env) = env::var("CHAINS") {
            if !chains_env.is_empty() {
                log::info!("CHAINS is set, relaying proofs");
                let access_token = env::var("PROVER_NETWORK_ACCESS_TOKEN")?;

                let chain_ids: Vec<u32> = Self::parse_env_var_array("CHAINS")?;
                let callbacks: Vec<String> = Self::parse_env_var_array("CALLBACKS")?;
                let callback_datas: Vec<String> = Self::parse_env_var_array("CALLBACK_DATAS")?;

                if !(chain_ids.len() == callbacks.len() && callbacks.len() == callback_datas.len())
                {
                    anyhow::bail!("CHAINS, CALLBACKS, and CALLBACK_DATAS must be of the same size");
                }

                let callbacks_refs: Vec<&str> = callbacks.iter().map(AsRef::as_ref).collect();
                let callback_datas_refs: Vec<&str> =
                    callback_datas.iter().map(AsRef::as_ref).collect();

                SP1Prover::relay_remote(
                    access_token,
                    &proof_id,
                    chain_ids,
                    callbacks_refs,
                    callback_datas_refs,
                )
                .await?;
                log::info!("Proofs relayed successfully");
            } else {
                log::info!("CHAINS is not set, skipping relay");
            }
        } else {
            log::info!("CHAINS environment variable is not set, no action required");
        }

        Ok(())
    }

    /// Generate a proof for the execution of the ELF with the given public inputs and a custom config.
    pub fn prove_with_config<SC: StarkGenericConfig>(
        elf: &[u8],
        stdin: SP1Stdin,
        config: SC,
    ) -> Result<SP1ProofWithIO<SC>>
    where
        SC: StarkGenericConfig,
        SC::Challenger: Clone,
        OpeningProof<SC>: Send + Sync,
        Com<SC>: Send + Sync,
        PcsProverData<SC>: Send + Sync,
        ShardMainData<SC>: Serialize + DeserializeOwned,
        SC::Val: p3_field::PrimeField32,
    {
        // If PROVER_NETWORK_ACCESS_TOKEN is set, prove remotely
        if std::env::var("PROVER_NETWORK_ACCESS_TOKEN").is_ok() {
            log::info!("PROVER_NETWORK_ACCESS_TOKEN is set, proving remotely");
            let proof_result = match tokio::runtime::Handle::try_current() {
                std::result::Result::Ok(handle) => tokio::task::block_in_place(|| {
                    handle.block_on(async { Self::prove_remote(elf, stdin).await })
                }),
                Err(_) => {
                    let rt = tokio::runtime::Runtime::new()?;
                    rt.block_on(async { Self::prove_remote(elf, stdin).await })
                }
            };

            match proof_result {
                std::result::Result::Ok((proof_with_io, Some(proof_id))) => {
                    let rt = tokio::runtime::Runtime::new().expect("Failed to create a Runtime");

                    rt.block_on(async { Self::relay_proof_if_required(proof_id).await })?;

                    Ok(proof_with_io)
                }
                _ => Err(anyhow::anyhow!("prove_remote failed")),
            }
        } else {
            let program = Program::from(elf);
            let (proof, public_values_vec) = run_and_prove(program, &stdin.buffer, config);
            let public_values = SP1PublicValues::from(&public_values_vec);
            Ok(SP1ProofWithIO {
                proof,
                stdin,
                public_values,
            })
        }
    }

    /// Return a comma separated list of values from the given environment variable.
    fn parse_env_var_array<T: FromStr>(env_var: &str) -> Result<Vec<T>> {
        let var_value = env::var(env_var)
            .with_context(|| format!("{} environment variable not set", env_var))?;

        var_value
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<T>())
            .collect::<Result<Vec<T>, T::Err>>()
            .map_err(|_| anyhow::anyhow!("Failed to parse one or more values in {}", env_var))
    }
}

impl SP1Verifier {
    /// Verify a proof generated by `SP1Prover`.
    #[allow(unused_variables)]
    pub fn verify(
        elf: &[u8],
        proof: &SP1ProofWithIO<BabyBearPoseidon2>,
    ) -> Result<(), ProgramVerificationError> {
        Self::verify_with_config(elf, proof, BabyBearPoseidon2::new())
    }

    /// Verify a proof generated by `SP1Prover` with a custom config.
    #[allow(unused_variables)]
    pub fn verify_with_config<SC: StarkGenericConfig>(
        elf: &[u8],
        proof: &SP1ProofWithIO<SC>,
        config: SC,
    ) -> Result<(), ProgramVerificationError>
    where
        SC: StarkGenericConfig,
        SC::Challenger: Clone,
        OpeningProof<SC>: Send + Sync,
        Com<SC>: Send + Sync,
        PcsProverData<SC>: Send + Sync,
        ShardMainData<SC>: Serialize + DeserializeOwned,
        SC::Val: p3_field::PrimeField32,
    {
        let mut challenger = config.challenger();
        let machine = RiscvAir::machine(config);

        let (_, vk) = machine.setup(&Program::from(elf));
        machine.verify(&vk, &proof.proof, &mut challenger)
    }
}

impl<SC: StarkGenericConfig + Serialize + DeserializeOwned> SP1ProofWithIO<SC> {
    /// Saves the proof as a JSON to the given path.
    pub fn save(&self, path: &str) -> Result<()> {
        let data = serde_json::to_string(self).unwrap();
        fs::write(path, data).unwrap();
        Ok(())
    }
}
