pub mod command;
pub mod config;
pub mod nats_client;

use config::Config;
use futures::StreamExt;
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
    let mut commands_sub = match nats_client.subscribe_commands().await {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to commands: {}", e);
            std::process::exit(1);
        }
    };

    info!("CLOUD_GATEWAY_CLIENT started for VIN={}", config.vin);

    // Await shutdown signal (SIGTERM/SIGINT)
    let shutdown = tokio::signal::ctrl_c();

    tokio::select! {
        _ = async {
            // Command processing loop placeholder - will be replaced in task group 3
            while let Some(msg) = commands_sub.next().await {
                info!(
                    "Received command on {}: {} bytes",
                    nats_client.commands_subject(),
                    msg.payload.len()
                );
            }
        } => {
            info!("Command subscription stream ended");
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
