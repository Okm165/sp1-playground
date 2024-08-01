// This file is @generated by prost-build.
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProveCoreRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ProveCoreResponse {
    #[prost(bytes = "vec", tag = "1")]
    pub result: ::prost::alloc::vec::Vec<u8>,
}
pub use twirp;
pub const SERVICE_FQN: &str = "/api.ProverService";
#[twirp::async_trait::async_trait]
pub trait ProverService {
    async fn prove_core(
        &self,
        ctx: twirp::Context,
        req: ProveCoreRequest,
    ) -> Result<ProveCoreResponse, twirp::TwirpErrorResponse>;
}
pub fn router<T>(api: std::sync::Arc<T>) -> twirp::Router
where
    T: ProverService + Send + Sync + 'static,
{
    twirp::details::TwirpRouterBuilder::new(api)
        .route(
            "/ProveCore",
            |api: std::sync::Arc<T>, ctx: twirp::Context, req: ProveCoreRequest| async move {
                api.prove_core(ctx, req).await
            },
        )
        .build()
}
#[twirp::async_trait::async_trait]
pub trait ProverServiceClient: Send + Sync + std::fmt::Debug {
    async fn prove_core(
        &self,
        req: ProveCoreRequest,
    ) -> Result<ProveCoreResponse, twirp::ClientError>;
}
#[twirp::async_trait::async_trait]
impl ProverServiceClient for twirp::client::Client {
    async fn prove_core(
        &self,
        req: ProveCoreRequest,
    ) -> Result<ProveCoreResponse, twirp::ClientError> {
        self.request("api.ProverService/ProveCore", req).await
    }
}
