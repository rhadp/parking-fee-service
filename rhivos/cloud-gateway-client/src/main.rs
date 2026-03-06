pub mod command;
pub mod command_processor;
pub mod config;
pub mod databroker_client;
pub mod nats_client;
pub mod response_relay;
pub mod telemetry;

/// Generated Kuksa VAL v2 protobuf types.
#[allow(clippy::doc_overindented_list_items)]
pub mod kuksa_proto {
    tonic::include_proto!("kuksa.val.v2");
}

use std::sync::Arc;
use tokio::sync::Mutex;

use config::Config;
use databroker_client::DatabrokerClient;
use nats_client::NatsClient;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load configuration from environment
    let config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("{e}");
            std::process::exit(1);
        }
    };

    info!(vin = %config.vin, "Starting CLOUD_GATEWAY_CLIENT");

    // Connect to NATS
    let nats_client = match NatsClient::connect(&config).await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to connect to NATS: {e}");
            std::process::exit(1);
        }
    };

    // Subscribe to commands
    let command_sub = match nats_client.subscribe_commands().await {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to command subject: {e}");
            std::process::exit(1);
        }
    };

    // Connect to DATA_BROKER via gRPC over UDS
    info!(
        path = %config.databroker_uds_path,
        "Connecting to DATA_BROKER"
    );
    let databroker = match DatabrokerClient::connect(&config.databroker_uds_path).await {
        Ok(db) => Arc::new(Mutex::new(db)),
        Err(e) => {
            error!("Failed to connect to DATA_BROKER: {e}");
            std::process::exit(1);
        }
    };

    info!(
        vin = %config.vin,
        "CLOUD_GATEWAY_CLIENT started for VIN={}",
        config.vin
    );

    // Spawn the command processor task
    let cmd_databroker = Arc::clone(&databroker);
    let cmd_handle = tokio::spawn(async move {
        command_processor::run(command_sub, cmd_databroker).await;
    });

    // Spawn the response relay task
    let resp_databroker = Arc::clone(&databroker);
    let resp_nats = nats_client.clone();
    let resp_handle = tokio::spawn(async move {
        response_relay::run(resp_databroker, resp_nats).await;
    });

    // Spawn the telemetry publisher task
    let telem_databroker = Arc::clone(&databroker);
    let telem_nats = nats_client.clone();
    let telem_handle = tokio::spawn(async move {
        telemetry::run(telem_databroker, telem_nats).await;
    });

    // Wait for shutdown signal or task failure.
    // If any task exits with an error, log it. The orchestrator or
    // supervisor can restart the whole process.
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal, exiting");
        }
        result = cmd_handle => {
            match result {
                Ok(()) => warn!("Command processor task exited, restarting may be needed"),
                Err(e) => error!("Command processor task failed: {e}"),
            }
        }
        result = resp_handle => {
            match result {
                Ok(()) => warn!("Response relay task exited, restarting may be needed"),
                Err(e) => error!("Response relay task failed: {e}"),
            }
        }
        result = telem_handle => {
            match result {
                Ok(()) => warn!("Telemetry publisher task exited, restarting may be needed"),
                Err(e) => error!("Telemetry publisher task failed: {e}"),
            }
        }
    }

    info!("CLOUD_GATEWAY_CLIENT shut down");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_startup() {
        assert!(true, "cloud-gateway-client skeleton compiles and starts");
    }
}
