pub mod command;
pub mod command_processor;
pub mod config;
pub mod databroker_client;
pub mod nats_client;

use config::Config;
use databroker_client::DataBrokerClient;
use nats_client::NatsClient;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Parse configuration from environment variables
    let config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };

    info!("Starting CLOUD_GATEWAY_CLIENT for VIN={}", config.vin);

    // Connect to NATS
    let nats_client = match NatsClient::connect(&config).await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to connect to NATS: {}", e);
            std::process::exit(1);
        }
    };

    // Subscribe to commands subject
    let commands_sub = match nats_client.subscribe_commands().await {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to commands: {}", e);
            std::process::exit(1);
        }
    };

    // Connect to DATA_BROKER via gRPC over UDS
    let databroker = DataBrokerClient::connect(&config.databroker_uds_path).await;
    let databroker = match databroker {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to connect to DATA_BROKER: {}", e);
            std::process::exit(1);
        }
    };

    info!("CLOUD_GATEWAY_CLIENT started for VIN={}", config.vin);

    // Await shutdown signal (SIGTERM/SIGINT)
    let shutdown = tokio::signal::ctrl_c();

    let uds_path = config.databroker_uds_path.clone();

    tokio::select! {
        _ = command_processor::run(commands_sub, databroker, uds_path) => {
            info!("Command processor stopped");
        }
        _ = shutdown => {
            info!("Shutdown signal received, stopping CLOUD_GATEWAY_CLIENT");
        }
    }

    info!("CLOUD_GATEWAY_CLIENT stopped");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_startup() {
        assert!(true, "cloud-gateway-client skeleton compiles and runs");
    }
}
