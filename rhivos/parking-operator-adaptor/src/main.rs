//! Parking Operator Adaptor skeleton.
//!
//! This service manages parking sessions via operator APIs.
//! In this skeleton, all RPCs return `UNIMPLEMENTED` (gRPC code 12).

use clap::Parser;
use tokio::signal;
use tonic::{Request, Response, Status};
use tracing::{error, info};

use parking_proto::services::adapter::parking_adapter_server::{
    ParkingAdapter, ParkingAdapterServer,
};
use parking_proto::services::adapter::{
    GetRateRequest, GetRateResponse, GetStatusRequest, GetStatusResponse,
    StartSessionRequest, StartSessionResponse, StopSessionRequest, StopSessionResponse,
};

/// RHIVOS Parking Operator Adaptor
#[derive(Parser, Debug)]
#[command(
    name = "parking-operator-adaptor",
    about = "RHIVOS parking operator adaptor skeleton"
)]
struct Args {
    /// Address to listen on
    #[arg(long, env = "LISTEN_ADDR", default_value = "0.0.0.0:50054")]
    listen_addr: String,
}

/// Stub implementation — all RPCs return UNIMPLEMENTED.
#[derive(Debug, Default)]
pub struct ParkingAdapterStub;

#[tonic::async_trait]
impl ParkingAdapter for ParkingAdapterStub {
    async fn start_session(
        &self,
        _request: Request<StartSessionRequest>,
    ) -> Result<Response<StartSessionResponse>, Status> {
        Err(Status::unimplemented("StartSession not yet implemented"))
    }

    async fn stop_session(
        &self,
        _request: Request<StopSessionRequest>,
    ) -> Result<Response<StopSessionResponse>, Status> {
        Err(Status::unimplemented("StopSession not yet implemented"))
    }

    async fn get_status(
        &self,
        _request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        Err(Status::unimplemented("GetStatus not yet implemented"))
    }

    async fn get_rate(
        &self,
        _request: Request<GetRateRequest>,
    ) -> Result<Response<GetRateResponse>, Status> {
        Err(Status::unimplemented("GetRate not yet implemented"))
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

    info!("parking-operator-adaptor starting on {}", addr);

    tonic::transport::Server::builder()
        .add_service(ParkingAdapterServer::new(ParkingAdapterStub))
        .serve_with_shutdown(addr, async {
            signal::ctrl_c()
                .await
                .expect("failed to listen for ctrl-c");
            info!("parking-operator-adaptor shutting down");
        })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_proto::common::VehicleId;
    use parking_proto::services::adapter::parking_adapter_client::ParkingAdapterClient;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    #[test]
    fn cli_parses_default_args() {
        let args = Args::parse_from(["parking-operator-adaptor"]);
        assert_eq!(args.listen_addr, "0.0.0.0:50054");
    }

    #[test]
    fn cli_parses_custom_listen_addr() {
        let args = Args::parse_from([
            "parking-operator-adaptor",
            "--listen-addr",
            "127.0.0.1:9999",
        ]);
        assert_eq!(args.listen_addr, "127.0.0.1:9999");
    }

    /// Start the stub gRPC server on a random port and return the address.
    async fn start_test_server() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(ParkingAdapterServer::new(ParkingAdapterStub))
                .serve_with_incoming(incoming)
                .await
                .unwrap();
        });

        addr
    }

    #[tokio::test]
    async fn start_session_returns_unimplemented() {
        let addr = start_test_server().await;
        let mut client =
            ParkingAdapterClient::connect(format!("http://{}", addr))
                .await
                .unwrap();

        let status = client
            .start_session(StartSessionRequest {
                vehicle_id: Some(VehicleId {
                    vin: "WBA12345678901234".into(),
                }),
                zone_id: "zone-a".into(),
                timestamp: 1700000000,
            })
            .await
            .unwrap_err();

        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn stop_session_returns_unimplemented() {
        let addr = start_test_server().await;
        let mut client =
            ParkingAdapterClient::connect(format!("http://{}", addr))
                .await
                .unwrap();

        let status = client
            .stop_session(StopSessionRequest {
                session_id: "session-1".into(),
                timestamp: 1700001000,
            })
            .await
            .unwrap_err();

        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn get_status_returns_unimplemented() {
        let addr = start_test_server().await;
        let mut client =
            ParkingAdapterClient::connect(format!("http://{}", addr))
                .await
                .unwrap();

        let status = client
            .get_status(GetStatusRequest {
                session_id: "session-1".into(),
            })
            .await
            .unwrap_err();

        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn get_rate_returns_unimplemented() {
        let addr = start_test_server().await;
        let mut client =
            ParkingAdapterClient::connect(format!("http://{}", addr))
                .await
                .unwrap();

        let status = client
            .get_rate(GetRateRequest {
                zone_id: "zone-a".into(),
            })
            .await
            .unwrap_err();

        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }
}
