//! Parking Operator Adaptor.
//!
//! This service manages parking sessions via operator APIs. It subscribes to
//! DATA_BROKER lock events and communicates with a PARKING_OPERATOR's REST API
//! to start and stop parking sessions automatically. It also exposes a gRPC
//! `ParkingAdapter` service for manual session control and status queries.
//!
//! # Requirements
//!
//! - 04-REQ-1.1: Subscribe to `IsLocked` on DATA_BROKER via gRPC streaming.
//! - 04-REQ-2.1: Expose gRPC server implementing `ParkingAdapter` service.
//! - 04-REQ-2.6: Accept configuration via environment variables.

pub mod config;
pub mod grpc_server;
pub mod lock_watcher;
pub mod operator_client;
pub mod session;

use std::sync::Arc;

use clap::Parser;
use config::Config;
use grpc_server::ParkingAdapterService;
use lock_watcher::SessionState;
use operator_client::OperatorClient;
use tokio::signal;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use parking_proto::kuksa_client::KuksaClient;
use parking_proto::services::adapter::parking_adapter_server::ParkingAdapterServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = Config::parse();

    let addr: std::net::SocketAddr = config.listen_addr.parse().map_err(|e| {
        error!("Invalid listen address '{}': {}", config.listen_addr, e);
        e
    })?;

    info!("parking-operator-adaptor starting on {}", addr);
    info!(
        "config: databroker={}, operator={}, zone={}, vin={}",
        config.databroker_addr, config.parking_operator_url, config.zone_id, config.vehicle_vin
    );

    // Shared session state between lock watcher and gRPC server
    let session_state: SessionState = Arc::new(Mutex::new(None));

    // Create REST client for the PARKING_OPERATOR
    let operator = OperatorClient::new(&config.parking_operator_url);

    // Connect to Kuksa Databroker
    let kuksa = match KuksaClient::connect(&config.databroker_addr).await {
        Ok(client) => {
            info!("connected to DATA_BROKER at {}", config.databroker_addr);
            Some(client)
        }
        Err(e) => {
            warn!(
                "failed to connect to DATA_BROKER at {}: {} — running without lock watcher",
                config.databroker_addr, e
            );
            None
        }
    };

    // Spawn lock watcher task if Kuksa is available
    if let Some(ref kuksa_client) = kuksa {
        let watcher_kuksa = kuksa_client.clone();
        let watcher_operator = operator.clone();
        let watcher_session = session_state.clone();
        let watcher_config = config.clone();

        tokio::spawn(async move {
            lock_watcher::watch_lock_events(
                watcher_kuksa,
                watcher_operator,
                watcher_session,
                watcher_config,
            )
            .await;
            warn!("lock watcher task exited");
        });
    }

    // Create gRPC server
    let service = ParkingAdapterService::new(
        session_state,
        operator,
        kuksa,
        config,
    );

    info!("gRPC server listening on {}", addr);

    tonic::transport::Server::builder()
        .add_service(ParkingAdapterServer::new(service))
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

    #[test]
    fn cli_parses_default_listen_addr() {
        let config = Config::parse_from([
            "parking-operator-adaptor",
            "--parking-operator-url",
            "http://op:8082",
            "--zone-id",
            "zone-1",
            "--vehicle-vin",
            "VIN1",
        ]);
        assert_eq!(config.listen_addr, "0.0.0.0:50054");
    }

    #[test]
    fn cli_parses_custom_listen_addr() {
        let config = Config::parse_from([
            "parking-operator-adaptor",
            "--listen-addr",
            "127.0.0.1:9999",
            "--parking-operator-url",
            "http://op:8082",
            "--zone-id",
            "zone-1",
            "--vehicle-vin",
            "VIN1",
        ]);
        assert_eq!(config.listen_addr, "127.0.0.1:9999");
    }
}
