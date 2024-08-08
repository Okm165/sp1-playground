use crate::common::types::{
    ChallengerType, CheckpointType, CommitmentType, PublicValueStreamType, PublicValuesType,
    RecordType,
};
use crate::ProveArgs;
use crate::{common, PublicValuesTuple};
use alloy_sol_types::SolType;
use anyhow::Result;
use p3_baby_bear::BabyBear;
use sp1_core::stark::MachineRecord;
use sp1_core::{
    runtime::Runtime,
    stark::{MachineProof, MachineProver, ShardProof, StarkGenericConfig},
    utils::{BabyBearPoseidon2, SP1CoreProverError},
};
use sp1_prover::{SP1CoreProof, SP1CoreProofData, SP1ProofWithMetadata};
use sp1_sdk::{SP1Proof, SP1ProofWithPublicValues, SP1PublicValues};

fn operator_split_into_checkpoints(
    runtime: &mut Runtime,
) -> Result<(PublicValueStreamType, PublicValuesType, Vec<CheckpointType>), SP1CoreProverError> {
    // Execute the program, saving checkpoints at the start of every `shard_batch_size` cycle range.
    let create_checkpoints_span = tracing::debug_span!("create checkpoints").entered();
    let mut checkpoints = Vec::new();
    let (public_values_stream, public_values) = loop {
        // Execute the runtime until we reach a checkpoint.
        let (checkpoint, done) = runtime
            .execute_state()
            .map_err(SP1CoreProverError::ExecutionError)?;

        // Save the checkpoint to a temp file.
        let mut checkpoint_file = tempfile::tempfile().map_err(SP1CoreProverError::IoError)?;
        checkpoint
            .save(&mut checkpoint_file)
            .map_err(SP1CoreProverError::IoError)?;
        checkpoints.push(checkpoint_file);

        // If we've reached the final checkpoint, break out of the loop.
        if done {
            break (
                runtime.state.public_values_stream.clone(),
                runtime
                    .records
                    .last()
                    .expect("at least one record")
                    .public_values,
            );
        }
    };
    create_checkpoints_span.exit();

    Ok((public_values_stream, public_values, checkpoints))
}

pub fn operator_split_into_checkpoints_impl(
    args: ProveArgs,
) -> Result<(
    PublicValueStreamType,
    PublicValuesType,
    Vec<CheckpointType>,
    u64,
)> {
    let (client, stdin, pk, _) = common::init_client(args.clone());
    let (program, core_opts, context) = common::bootstrap(&client, &pk).unwrap();
    tracing::info!("Program size = {}", program.instructions.len());

    // Execute the program.
    let mut runtime = common::build_runtime(program, &stdin, core_opts, context);

    let (public_values_stream, public_values, checkpoints) =
        operator_split_into_checkpoints(&mut runtime).unwrap();

    Ok((
        public_values_stream,
        public_values,
        checkpoints,
        runtime.state.global_clk,
    ))
}

pub fn operator_absorb_commits_impl(
    args: ProveArgs,
    commitments_vec: Vec<Vec<CommitmentType>>,
    records_vec: Vec<Vec<RecordType>>,
) -> Result<ChallengerType> {
    if commitments_vec.len() != records_vec.len() {
        return Err(anyhow::anyhow!(
            "commitments_vec and records_vec must have the same length"
        ));
    }
    let (client, stdin, pk, _) = common::init_client(args.clone());
    let (program, core_opts, context) = common::bootstrap(&client, &pk).unwrap();

    // Execute the program.
    let runtime = common::build_runtime(program, &stdin, core_opts, context);

    // Setup the machine.
    let (_, stark_vk) = client
        .prover
        .sp1_prover()
        .core_prover
        .setup(runtime.program.as_ref());

    let mut challenger = client.prover.sp1_prover().core_prover.config().challenger();
    stark_vk.observe_into(&mut challenger);

    for (commitments, records) in commitments_vec.into_iter().zip(records_vec.into_iter()) {
        for (commitment, record) in commitments.into_iter().zip(records.into_iter()) {
            client.prover.sp1_prover().core_prover.update(
                &mut challenger,
                commitment,
                &record.public_values::<BabyBear>()[0..client
                    .prover
                    .sp1_prover()
                    .core_prover
                    .machine()
                    .num_pv_elts()],
            );
        }
    }

    Ok(challenger)
}

pub fn construct_sp1_core_proof_impl(
    args: ProveArgs,
    shard_proofs_vec: Vec<Vec<ShardProof<BabyBearPoseidon2>>>,
    public_values_stream: PublicValueStreamType,
    cycles: u64,
) -> Result<SP1ProofWithMetadata<SP1CoreProofData>> {
    let (_, stdin, _, _) = common::init_client(args.clone());

    let shard_proofs = shard_proofs_vec
        .into_iter()
        .flat_map(|vec| vec.into_iter())
        .collect();

    let proof = MachineProof { shard_proofs };

    tracing::info!(
        "summary: proofSize={}",
        bincode::serialize(&proof).unwrap().len(),
    );

    let public_values = SP1PublicValues::from(&public_values_stream);
    let sp1_core_proof = SP1CoreProof {
        proof: SP1CoreProofData(proof.shard_proofs),
        stdin: stdin.clone(),
        public_values,
        cycles,
    };

    Ok(sp1_core_proof)
}

pub fn construct_core_proof(
    args: &Vec<u8>,
    core_proof: &Vec<u8>,
) -> Result<SP1ProofWithPublicValues> {
    let args_obj = ProveArgs::from_slice(args.as_slice());
    let core_proof_obj: SP1CoreProof = bincode::deserialize(core_proof).unwrap();

    let (client, _, _, vk) = common::init_client(args_obj);

    let proof = SP1ProofWithPublicValues {
        proof: SP1Proof::Core(core_proof_obj.proof.0),
        stdin: core_proof_obj.stdin,
        public_values: core_proof_obj.public_values,
        sp1_version: client.prover.version().to_string(),
    };

    client.verify(&proof, &vk).expect("failed to verify proof");
    tracing::info!("Successfully generated core-proof(verified)");

    let (_, _, fib_n) =
        PublicValuesTuple::abi_decode(proof.public_values.as_slice(), false).unwrap();
    tracing::info!("Public Input: {}", fib_n);

    Ok(proof)
}
