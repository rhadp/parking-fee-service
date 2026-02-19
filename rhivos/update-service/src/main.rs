//! Update Service — manages adapter container lifecycle.
//!
//! This service manages adapter lifecycle operations (install, remove, status)
//! via gRPC. The state machine and persistence are fully implemented; gRPC
//! server RPCs remain stubs until task group 6.

pub mod config;
pub mod state;

use clap::Parser;
use tokio::signal;
use tonic::{Request, Response, Status};
use tracing::{error, info};

use parking_proto::services::update::update_service_server::{
    UpdateService, UpdateServiceServer,
};
use parking_proto::services::update::{
    AdapterStateEvent, GetAdapterStatusRequest, GetAdapterStatusResponse,
    InstallAdapterRequest, InstallAdapterResponse, ListAdaptersRequest,
    ListAdaptersResponse, RemoveAdapterRequest, RemoveAdapterResponse,
    WatchAdapterStatesRequest,
};

/// Stub implementation — all RPCs return UNIMPLEMENTED.
///
/// This will be replaced in task group 6 with a real implementation
/// backed by the state machine and podman wrapper.
#[derive(Debug, Default)]
pub struct UpdateServiceStub;

#[tonic::async_trait]
impl UpdateService for UpdateServiceStub {
    async fn install_adapter(
        &self,
        _request: Request<InstallAdapterRequest>,
    ) -> Result<Response<InstallAdapterResponse>, Status> {
        Err(Status::unimplemented("InstallAdapter not yet implemented"))
    }

    type WatchAdapterStatesStream =
        tokio_stream::wrappers::ReceiverStream<Result<AdapterStateEvent, Status>>;

    async fn watch_adapter_states(
        &self,
        _request: Request<WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        Err(Status::unimplemented(
            "WatchAdapterStates not yet implemented",
        ))
    }

    async fn list_adapters(
        &self,
        _request: Request<ListAdaptersRequest>,
    ) -> Result<Response<ListAdaptersResponse>, Status> {
        Err(Status::unimplemented("ListAdapters not yet implemented"))
    }

    async fn remove_adapter(
        &self,
        _request: Request<RemoveAdapterRequest>,
    ) -> Result<Response<RemoveAdapterResponse>, Status> {
        Err(Status::unimplemented("RemoveAdapter not yet implemented"))
    }

    async fn get_adapter_status(
        &self,
        _request: Request<GetAdapterStatusRequest>,
    ) -> Result<Response<GetAdapterStatusResponse>, Status> {
        Err(Status::unimplemented(
            "GetAdapterStatus not yet implemented",
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cfg = config::Config::parse();

    let addr: std::net::SocketAddr = cfg.listen_addr.parse().map_err(|e| {
        error!("Invalid listen address '{}': {}", cfg.listen_addr, e);
        e
    })?;

    // Validate offload timeout at startup
    let offload_duration = cfg.offload_duration().map_err(|e| {
        error!("Invalid OFFLOAD_TIMEOUT '{}': {}", cfg.offload_timeout, e);
        e
    })?;

    info!(
        listen_addr = %addr,
        data_dir = %cfg.data_dir,
        offload_timeout = ?offload_duration,
        "update-service starting"
    );

    // Load persisted adapter state
    let _store = state::AdapterStore::load(&cfg.data_dir).map_err(|e| {
        error!("Failed to load adapter state: {}", e);
        e
    })?;

    tonic::transport::Server::builder()
        .add_service(UpdateServiceServer::new(UpdateServiceStub))
        .serve_with_shutdown(addr, async {
            signal::ctrl_c()
                .await
                .expect("failed to listen for ctrl-c");
            info!("update-service shutting down");
        })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::Config;
    use parking_proto::services::update::update_service_client::UpdateServiceClient;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    #[test]
    fn cli_parses_default_args() {
        let cfg = Config::parse_from(["update-service"]);
        assert_eq!(cfg.listen_addr, "0.0.0.0:50053");
    }

    #[test]
    fn cli_parses_custom_listen_addr() {
        let cfg = Config::parse_from(["update-service", "--listen-addr", "127.0.0.1:9999"]);
        assert_eq!(cfg.listen_addr, "127.0.0.1:9999");
    }

    /// Start the stub gRPC server on a random port and return the address.
    async fn start_test_server() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(UpdateServiceServer::new(UpdateServiceStub))
                .serve_with_incoming(incoming)
                .await
                .unwrap();
        });

        addr
    }

    #[tokio::test]
    async fn install_adapter_returns_unimplemented() {
        let addr = start_test_server().await;
        let mut client =
            UpdateServiceClient::connect(format!("http://{}", addr))
                .await
                .unwrap();

        let status = client
            .install_adapter(InstallAdapterRequest {
                image_ref: "test:latest".into(),
                checksum: "sha256:abc".into(),
            })
            .await
            .unwrap_err();

        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn watch_adapter_states_returns_unimplemented() {
        let addr = start_test_server().await;
        let mut client =
            UpdateServiceClient::connect(format!("http://{}", addr))
                .await
                .unwrap();

        let status = client
            .watch_adapter_states(WatchAdapterStatesRequest {})
            .await
            .unwrap_err();

        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn list_adapters_returns_unimplemented() {
        let addr = start_test_server().await;
        let mut client =
            UpdateServiceClient::connect(format!("http://{}", addr))
                .await
                .unwrap();

        let status = client
            .list_adapters(ListAdaptersRequest {})
            .await
            .unwrap_err();

        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn remove_adapter_returns_unimplemented() {
        let addr = start_test_server().await;
        let mut client =
            UpdateServiceClient::connect(format!("http://{}", addr))
                .await
                .unwrap();

        let status = client
            .remove_adapter(RemoveAdapterRequest {
                adapter_id: "test-adapter".into(),
            })
            .await
            .unwrap_err();

        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn get_adapter_status_returns_unimplemented() {
        let addr = start_test_server().await;
        let mut client =
            UpdateServiceClient::connect(format!("http://{}", addr))
                .await
                .unwrap();

        let status = client
            .get_adapter_status(GetAdapterStatusRequest {
                adapter_id: "test-adapter".into(),
            })
            .await
            .unwrap_err();

        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }
}
