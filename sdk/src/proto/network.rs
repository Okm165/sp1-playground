// This file is @generated by prost-build.
/// The request to create a proof, the first step in requesting a proof.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateProofRequest {
    /// The signature of the message.
    #[prost(bytes = "vec", tag = "1")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    /// The nonce for the account.
    #[prost(uint64, tag = "2")]
    pub nonce: u64,
    /// The mode for proof generation.
    #[prost(enumeration = "ProofMode", tag = "3")]
    pub mode: i32,
    /// The deadline for the proof request, signifying the latest time a fulfillment would be valid.
    #[prost(uint64, tag = "4")]
    pub deadline: u64,
}
/// The response for creating a proof.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateProofResponse {
    /// The proof identifier.
    #[prost(string, tag = "1")]
    pub proof_id: ::prost::alloc::string::String,
    /// The URL to upload the ELF file.
    #[prost(string, tag = "2")]
    pub program_url: ::prost::alloc::string::String,
    /// The URL to upload the standard input (stdin).
    #[prost(string, tag = "3")]
    pub stdin_url: ::prost::alloc::string::String,
}
/// The request to submit a proof, the second step in requesting a proof. MUST be called when the
/// proof is in a PROOF_REQUESTED state and MUST be called after uploading the program and stdin to
/// the URLs provided during create proof.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubmitProofRequest {
    /// The signature of the message.
    #[prost(bytes = "vec", tag = "1")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    /// The nonce for the account.
    #[prost(uint64, tag = "2")]
    pub nonce: u64,
    /// The proof identifier.
    #[prost(string, tag = "3")]
    pub proof_id: ::prost::alloc::string::String,
}
/// The response for submitting a proof, empty on success.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubmitProofResponse {}
/// The request to claim a proof, which agrees to fulfill the proof by the deadline. MUST be called
/// when the proof is in a PROOF_REQUESTED state.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClaimProofRequest {
    /// The signature of the message.
    #[prost(bytes = "vec", tag = "1")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    /// The nonce for the account.
    #[prost(uint64, tag = "2")]
    pub nonce: u64,
    /// The proof identifier.
    #[prost(string, tag = "3")]
    pub proof_id: ::prost::alloc::string::String,
}
/// The response for claiming a proof, giving identifiers for the locations to retrieve the program
/// and stdin, as well as the location to upload the proof.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClaimProofResponse {
    /// The artifact identifier for the program location.
    #[prost(string, tag = "1")]
    pub program_artifact_id: ::prost::alloc::string::String,
    /// The artifact identifier for the stdin location.
    #[prost(string, tag = "2")]
    pub stdin_artifact_id: ::prost::alloc::string::String,
    /// The artifact identifier for the proof location.
    #[prost(string, tag = "3")]
    pub proof_artifact_id: ::prost::alloc::string::String,
}
/// The request to fulfill a proof. MUST be called after the proof has been uploaded and MUST be called
/// when the proof is in a PROOF_CLAIMED state.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FulfillProofRequest {
    /// The signature of the message.
    #[prost(bytes = "vec", tag = "1")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    /// The nonce for the account.
    #[prost(uint64, tag = "2")]
    pub nonce: u64,
    /// The proof identifier.
    #[prost(string, tag = "3")]
    pub proof_id: ::prost::alloc::string::String,
}
/// The response for fulfilling a proof, empty on success.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FulfillProofResponse {
    /// The amount of time, in seconds, between proof claim and fulfillment.
    #[prost(uint64, tag = "1")]
    pub proving_seconds: u64,
}
/// The request to relay a proof through the NetworkGateway on a given chain. MUST be called when the
/// proof is in a PROOF_FULFILLED state.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RelayProofRequest {
    /// The signature of the message.
    #[prost(bytes = "vec", tag = "1")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    /// The nonce for the account.
    #[prost(uint64, tag = "2")]
    pub nonce: u64,
    /// The proof identifier.
    #[prost(string, tag = "3")]
    pub proof_id: ::prost::alloc::string::String,
    /// The chain ID for the requested chain.
    #[prost(uint32, tag = "4")]
    pub chain_id: u32,
    /// The address of the verifier for this proof.
    #[prost(bytes = "vec", tag = "5")]
    pub verifier: ::prost::alloc::vec::Vec<u8>,
    /// The address of the callback to call after the proof has been verified by the verifier.
    #[prost(bytes = "vec", tag = "6")]
    pub callback: ::prost::alloc::vec::Vec<u8>,
    /// The data to send to the callback, including the function selector.
    #[prost(bytes = "vec", tag = "7")]
    pub callback_data: ::prost::alloc::vec::Vec<u8>,
}
/// The response for relaying a proof.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RelayProofResponse {
    /// The transaction identifier.
    #[prost(string, tag = "1")]
    pub tx_id: ::prost::alloc::string::String,
}
/// The request for an account nonce. Used to check current nonce for the account, which must match when signing and sending a message.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetNonceRequest {
    /// The account's address for which to get the nonce.
    #[prost(bytes = "vec", tag = "1")]
    pub address: ::prost::alloc::vec::Vec<u8>,
}
/// The response for a nonce request.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetNonceResponse {
    /// The nonce for the given address. It should be signed along with the rest of the message.
    #[prost(uint64, tag = "1")]
    pub nonce: u64,
}
/// The request to get a proof status by a given proof ID.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetProofStatusRequest {
    /// The proof identifier.
    #[prost(string, tag = "1")]
    pub proof_id: ::prost::alloc::string::String,
}
/// The response for a proof status request.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetProofStatusResponse {
    /// The status of the proof request.
    #[prost(enumeration = "ProofStatus", tag = "1")]
    pub status: i32,
    /// Optional proof URL, where you can download the result of the proof request. Only included if
    /// the proof has been fulfilled.
    #[prost(string, tag = "2")]
    pub proof_url: ::prost::alloc::string::String,
}
/// The request to get proof requests by a given status.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetProofRequestsRequest {
    /// The status of the proof requests to get.
    #[prost(enumeration = "ProofStatus", tag = "1")]
    pub status: i32,
}
/// A proof request.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RequestedProof {
    /// The proof identifier.
    #[prost(string, tag = "1")]
    pub proof_id: ::prost::alloc::string::String,
    /// The mode for proof generation.
    #[prost(enumeration = "ProofMode", tag = "2")]
    pub mode: i32,
}
/// The response for getting proof requests by a given status.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetProofRequestsResponse {
    /// The proof identifiers of the proof requests. Limited to the 10 most recent proof requests with
    /// that status.
    #[prost(message, repeated, tag = "1")]
    pub proofs: ::prost::alloc::vec::Vec<RequestedProof>,
}
/// The request to get the status of a relay request.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetRelayStatusRequest {
    /// The transaction identifier.
    #[prost(string, tag = "1")]
    pub tx_id: ::prost::alloc::string::String,
}
/// The response for getting the status of a relay request.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetRelayStatusResponse {
    /// The status of the transaction.
    #[prost(enumeration = "TransactionStatus", tag = "1")]
    pub status: i32,
    /// The transaction hash.
    #[prost(bytes = "vec", tag = "2")]
    pub tx_hash: ::prost::alloc::vec::Vec<u8>,
    /// The transactionsimulation URL, only present if the transaction failed.
    #[prost(string, tag = "3")]
    pub simulation_url: ::prost::alloc::string::String,
}
/// The mode used when generating the proof.
#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    ::prost::Enumeration,
)]
#[repr(i32)]
pub enum ProofMode {
    /// Unspecified or invalid proof mode.
    Unspecified = 0,
    /// The proof mode for an SP1 core proof.
    Core = 1,
    /// The proof mode for a compressed proof.
    Compressed = 2,
    /// The proof mode for a PlonK proof.
    Plonk = 3,
}
impl ProofMode {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            ProofMode::Unspecified => "PROOF_MODE_UNSPECIFIED",
            ProofMode::Core => "PROOF_MODE_CORE",
            ProofMode::Compressed => "PROOF_MODE_COMPRESSED",
            ProofMode::Plonk => "PROOF_MODE_PLONK",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "PROOF_MODE_UNSPECIFIED" => Some(Self::Unspecified),
            "PROOF_MODE_CORE" => Some(Self::Core),
            "PROOF_MODE_COMPRESSED" => Some(Self::Compressed),
            "PROOF_MODE_PLONK" => Some(Self::Plonk),
            _ => None,
        }
    }
}
/// The status of a proof request.
#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    ::prost::Enumeration,
)]
#[repr(i32)]
pub enum ProofStatus {
    /// Unspecified or invalid status.
    ProofUnspecifiedStatus = 0,
    /// The proof request has been created but is awaiting the requester to submit it.
    ProofPreparing = 1,
    /// The proof request has been submitted and is awaiting a prover to claim it.
    ProofRequested = 2,
    /// The proof request has been claimed and is awaiting a prover to fulfill it.
    ProofClaimed = 3,
    /// The proof request has been fulfilled and is available for download.
    ProofFulfilled = 4,
    /// The proof request failed and will need to be re-created.
    ProofFailed = 5,
}
impl ProofStatus {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            ProofStatus::ProofUnspecifiedStatus => "PROOF_UNSPECIFIED_STATUS",
            ProofStatus::ProofPreparing => "PROOF_PREPARING",
            ProofStatus::ProofRequested => "PROOF_REQUESTED",
            ProofStatus::ProofClaimed => "PROOF_CLAIMED",
            ProofStatus::ProofFulfilled => "PROOF_FULFILLED",
            ProofStatus::ProofFailed => "PROOF_FAILED",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "PROOF_UNSPECIFIED_STATUS" => Some(Self::ProofUnspecifiedStatus),
            "PROOF_PREPARING" => Some(Self::ProofPreparing),
            "PROOF_REQUESTED" => Some(Self::ProofRequested),
            "PROOF_CLAIMED" => Some(Self::ProofClaimed),
            "PROOF_FULFILLED" => Some(Self::ProofFulfilled),
            "PROOF_FAILED" => Some(Self::ProofFailed),
            _ => None,
        }
    }
}
/// The status of a relay request transaction.
#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    ::prost::Enumeration,
)]
#[repr(i32)]
pub enum TransactionStatus {
    /// Unspecified or invalid status.
    TransactionUnspecifiedStatus = 0,
    /// The transaction has been scheduled for relay.
    TransactionScheduled = 1,
    /// The transaction has been broadcast to the requested chain.
    TransactionBroadcasted = 2,
    /// The transaction was never confirmed as mined.
    TransactionTimedout = 3,
    /// The transaction failed to be broadcast, likely due to a revert in simulation.
    TransactionFailed = 4,
    /// The transaction was mined successfully.
    TransactionFinalized = 5,
}
impl TransactionStatus {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            TransactionStatus::TransactionUnspecifiedStatus => "TRANSACTION_UNSPECIFIED_STATUS",
            TransactionStatus::TransactionScheduled => "TRANSACTION_SCHEDULED",
            TransactionStatus::TransactionBroadcasted => "TRANSACTION_BROADCASTED",
            TransactionStatus::TransactionTimedout => "TRANSACTION_TIMEDOUT",
            TransactionStatus::TransactionFailed => "TRANSACTION_FAILED",
            TransactionStatus::TransactionFinalized => "TRANSACTION_FINALIZED",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "TRANSACTION_UNSPECIFIED_STATUS" => Some(Self::TransactionUnspecifiedStatus),
            "TRANSACTION_SCHEDULED" => Some(Self::TransactionScheduled),
            "TRANSACTION_BROADCASTED" => Some(Self::TransactionBroadcasted),
            "TRANSACTION_TIMEDOUT" => Some(Self::TransactionTimedout),
            "TRANSACTION_FAILED" => Some(Self::TransactionFailed),
            "TRANSACTION_FINALIZED" => Some(Self::TransactionFinalized),
            _ => None,
        }
    }
}
pub const SERVICE_FQN: &str = "/network.NetworkService";
#[twirp::async_trait::async_trait]
pub trait NetworkService {
    async fn create_proof(
        &self,
        ctx: twirp::Context,
        req: CreateProofRequest,
    ) -> Result<CreateProofResponse, twirp::TwirpErrorResponse>;
    async fn submit_proof(
        &self,
        ctx: twirp::Context,
        req: SubmitProofRequest,
    ) -> Result<SubmitProofResponse, twirp::TwirpErrorResponse>;
    async fn claim_proof(
        &self,
        ctx: twirp::Context,
        req: ClaimProofRequest,
    ) -> Result<ClaimProofResponse, twirp::TwirpErrorResponse>;
    async fn fulfill_proof(
        &self,
        ctx: twirp::Context,
        req: FulfillProofRequest,
    ) -> Result<FulfillProofResponse, twirp::TwirpErrorResponse>;
    async fn relay_proof(
        &self,
        ctx: twirp::Context,
        req: RelayProofRequest,
    ) -> Result<RelayProofResponse, twirp::TwirpErrorResponse>;
    async fn get_nonce(
        &self,
        ctx: twirp::Context,
        req: GetNonceRequest,
    ) -> Result<GetNonceResponse, twirp::TwirpErrorResponse>;
    async fn get_proof_status(
        &self,
        ctx: twirp::Context,
        req: GetProofStatusRequest,
    ) -> Result<GetProofStatusResponse, twirp::TwirpErrorResponse>;
    async fn get_proof_requests(
        &self,
        ctx: twirp::Context,
        req: GetProofRequestsRequest,
    ) -> Result<GetProofRequestsResponse, twirp::TwirpErrorResponse>;
    async fn get_relay_status(
        &self,
        ctx: twirp::Context,
        req: GetRelayStatusRequest,
    ) -> Result<GetRelayStatusResponse, twirp::TwirpErrorResponse>;
}
pub fn router<T>(api: std::sync::Arc<T>) -> twirp::Router
where
    T: NetworkService + Send + Sync + 'static,
{
    twirp::details::TwirpRouterBuilder::new(api)
        .route(
            "/CreateProof",
            |api: std::sync::Arc<T>, ctx: twirp::Context, req: CreateProofRequest| async move {
                api.create_proof(ctx, req).await
            },
        )
        .route(
            "/SubmitProof",
            |api: std::sync::Arc<T>, ctx: twirp::Context, req: SubmitProofRequest| async move {
                api.submit_proof(ctx, req).await
            },
        )
        .route(
            "/ClaimProof",
            |api: std::sync::Arc<T>, ctx: twirp::Context, req: ClaimProofRequest| async move {
                api.claim_proof(ctx, req).await
            },
        )
        .route(
            "/FulfillProof",
            |api: std::sync::Arc<T>, ctx: twirp::Context, req: FulfillProofRequest| async move {
                api.fulfill_proof(ctx, req).await
            },
        )
        .route(
            "/RelayProof",
            |api: std::sync::Arc<T>, ctx: twirp::Context, req: RelayProofRequest| async move {
                api.relay_proof(ctx, req).await
            },
        )
        .route(
            "/GetNonce",
            |api: std::sync::Arc<T>, ctx: twirp::Context, req: GetNonceRequest| async move {
                api.get_nonce(ctx, req).await
            },
        )
        .route(
            "/GetProofStatus",
            |api: std::sync::Arc<T>, ctx: twirp::Context, req: GetProofStatusRequest| async move {
                api.get_proof_status(ctx, req).await
            },
        )
        .route(
            "/GetProofRequests",
            |api: std::sync::Arc<T>, ctx: twirp::Context, req: GetProofRequestsRequest| async move {
                api.get_proof_requests(ctx, req).await
            },
        )
        .route(
            "/GetRelayStatus",
            |api: std::sync::Arc<T>, ctx: twirp::Context, req: GetRelayStatusRequest| async move {
                api.get_relay_status(ctx, req).await
            },
        )
        .build()
}
#[twirp::async_trait::async_trait]
pub trait NetworkServiceClient: Send + Sync + std::fmt::Debug {
    async fn create_proof(
        &self,
        req: CreateProofRequest,
    ) -> Result<CreateProofResponse, twirp::ClientError>;
    async fn submit_proof(
        &self,
        req: SubmitProofRequest,
    ) -> Result<SubmitProofResponse, twirp::ClientError>;
    async fn claim_proof(
        &self,
        req: ClaimProofRequest,
    ) -> Result<ClaimProofResponse, twirp::ClientError>;
    async fn fulfill_proof(
        &self,
        req: FulfillProofRequest,
    ) -> Result<FulfillProofResponse, twirp::ClientError>;
    async fn relay_proof(
        &self,
        req: RelayProofRequest,
    ) -> Result<RelayProofResponse, twirp::ClientError>;
    async fn get_nonce(&self, req: GetNonceRequest)
        -> Result<GetNonceResponse, twirp::ClientError>;
    async fn get_proof_status(
        &self,
        req: GetProofStatusRequest,
    ) -> Result<GetProofStatusResponse, twirp::ClientError>;
    async fn get_proof_requests(
        &self,
        req: GetProofRequestsRequest,
    ) -> Result<GetProofRequestsResponse, twirp::ClientError>;
    async fn get_relay_status(
        &self,
        req: GetRelayStatusRequest,
    ) -> Result<GetRelayStatusResponse, twirp::ClientError>;
}
#[twirp::async_trait::async_trait]
impl NetworkServiceClient for twirp::client::Client {
    async fn create_proof(
        &self,
        req: CreateProofRequest,
    ) -> Result<CreateProofResponse, twirp::ClientError> {
        let url = self.base_url.join("network.NetworkService/CreateProof")?;
        self.request(url, req).await
    }
    async fn submit_proof(
        &self,
        req: SubmitProofRequest,
    ) -> Result<SubmitProofResponse, twirp::ClientError> {
        let url = self.base_url.join("network.NetworkService/SubmitProof")?;
        self.request(url, req).await
    }
    async fn claim_proof(
        &self,
        req: ClaimProofRequest,
    ) -> Result<ClaimProofResponse, twirp::ClientError> {
        let url = self.base_url.join("network.NetworkService/ClaimProof")?;
        self.request(url, req).await
    }
    async fn fulfill_proof(
        &self,
        req: FulfillProofRequest,
    ) -> Result<FulfillProofResponse, twirp::ClientError> {
        let url = self.base_url.join("network.NetworkService/FulfillProof")?;
        self.request(url, req).await
    }
    async fn relay_proof(
        &self,
        req: RelayProofRequest,
    ) -> Result<RelayProofResponse, twirp::ClientError> {
        let url = self.base_url.join("network.NetworkService/RelayProof")?;
        self.request(url, req).await
    }
    async fn get_nonce(
        &self,
        req: GetNonceRequest,
    ) -> Result<GetNonceResponse, twirp::ClientError> {
        let url = self.base_url.join("network.NetworkService/GetNonce")?;
        self.request(url, req).await
    }
    async fn get_proof_status(
        &self,
        req: GetProofStatusRequest,
    ) -> Result<GetProofStatusResponse, twirp::ClientError> {
        let url = self
            .base_url
            .join("network.NetworkService/GetProofStatus")?;
        self.request(url, req).await
    }
    async fn get_proof_requests(
        &self,
        req: GetProofRequestsRequest,
    ) -> Result<GetProofRequestsResponse, twirp::ClientError> {
        let url = self
            .base_url
            .join("network.NetworkService/GetProofRequests")?;
        self.request(url, req).await
    }
    async fn get_relay_status(
        &self,
        req: GetRelayStatusRequest,
    ) -> Result<GetRelayStatusResponse, twirp::ClientError> {
        let url = self
            .base_url
            .join("network.NetworkService/GetRelayStatus")?;
        self.request(url, req).await
    }
}
