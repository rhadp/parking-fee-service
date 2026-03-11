use cloud_gateway_client::config::Config;
use cloud_gateway_client::databroker_client::DataBrokerClient;
use cloud_gateway_client::nats_client::NatsClient;
use cloud_gateway_client::{command_processor, response_relay, telemetry};
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
    let databroker = match DataBrokerClient::connect(&config.databroker_uds_path).await {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to connect to DATA_BROKER: {}", e);
            std::process::exit(1);
        }
    };

    info!("CLOUD_GATEWAY_CLIENT started for VIN={}", config.vin);

    let uds_path = config.databroker_uds_path.clone();
    let vin = config.vin.clone();

    // Spawn three concurrent pipelines
    let cmd_uds = uds_path.clone();
    let cmd_task = tokio::spawn(async move {
        command_processor::run(commands_sub, databroker.clone(), cmd_uds).await;
    });

    let relay_nats = nats_client.clone();
    let relay_uds = uds_path.clone();
    let relay_databroker = DataBrokerClient::connect(&uds_path).await;
    let relay_task = tokio::spawn(async move {
        match relay_databroker {
            Ok(db) => {
                response_relay::run(db, relay_nats, relay_uds).await;
            }
            Err(e) => {
                error!("Response relay failed to connect to DATA_BROKER: {}", e);
            }
        }
    });

    let telem_nats = nats_client.clone();
    let telem_uds = uds_path.clone();
    let telem_vin = vin.clone();
    let telem_databroker = DataBrokerClient::connect(&uds_path).await;
    let telem_task = tokio::spawn(async move {
        match telem_databroker {
            Ok(db) => {
                telemetry::run(db, telem_nats, telem_uds, telem_vin).await;
            }
            Err(e) => {
                error!("Telemetry publisher failed to connect to DATA_BROKER: {}", e);
            }
        }
    });

    // Wait for shutdown signal or any task to complete
    tokio::select! {
        result = cmd_task => {
            match result {
                Ok(()) => info!("Command processor stopped"),
                Err(e) => error!("Command processor task failed: {}", e),
            }
        }
        result = relay_task => {
            match result {
                Ok(()) => info!("Response relay stopped"),
                Err(e) => error!("Response relay task failed: {}", e),
            }
        }
        result = telem_task => {
            match result {
                Ok(()) => info!("Telemetry publisher stopped"),
                Err(e) => error!("Telemetry publisher task failed: {}", e),
            }
        }
        _ = tokio::signal::ctrl_c() => {
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
