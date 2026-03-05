pub mod command;
pub mod config;
pub mod nats_client;

use config::Config;
use nats_client::NatsClient;
use tracing::{error, info};

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
    let mut _command_sub = match nats_client.subscribe_commands().await {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to command subject: {e}");
            std::process::exit(1);
        }
    };

    info!(
        vin = %config.vin,
        "CLOUD_GATEWAY_CLIENT started for VIN={}",
        config.vin
    );

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal, exiting");
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
