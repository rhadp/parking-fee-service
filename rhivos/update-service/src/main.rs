//! Update Service skeleton.
//!
//! This service manages adapter lifecycle operations (install, remove, status).
//! In this skeleton, all RPCs return `UNIMPLEMENTED` (gRPC code 12).

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

/// RHIVOS Update Service
#[derive(Parser, Debug)]
#[command(name = "update-service", about = "RHIVOS update service skeleton")]
struct Args {
    /// Address to listen on
    #[arg(long, env = "LISTEN_ADDR", default_value = "0.0.0.0:50053")]
    listen_addr: String,
}

/// Stub implementation — all RPCs return UNIMPLEMENTED.
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

    let args = Args::parse();

    let addr: std::net::SocketAddr = args.listen_addr.parse().map_err(|e| {
        error!("Invalid listen address '{}': {}", args.listen_addr, e);
        e
    })?;

    info!("update-service starting on {}", addr);

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
    use parking_proto::services::update::update_service_client::UpdateServiceClient;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    #[test]
    fn cli_parses_default_args() {
        let args = Args::parse_from(["update-service"]);
        assert_eq!(args.listen_addr, "0.0.0.0:50053");
    }

    #[test]
    fn cli_parses_custom_listen_addr() {
        let args = Args::parse_from(["update-service", "--listen-addr", "127.0.0.1:9999"]);
        assert_eq!(args.listen_addr, "127.0.0.1:9999");
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
