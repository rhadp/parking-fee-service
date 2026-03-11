use locking_service::config::Config;
use locking_service::databroker_client::DataBrokerClient;
use locking_service::COMMAND_SIGNAL;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    // Initialize structured logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // 1. Parse configuration.
    let config = Config::from_env();
    info!(
        uds_path = %config.databroker_uds_path,
        "Loaded configuration"
    );

    // 2. Connect to DATA_BROKER with retry.
    let mut client = match DataBrokerClient::connect(&config.databroker_uds_path).await {
        Ok(c) => c,
        Err(e) => {
            error!("Fatal: could not connect to DATA_BROKER: {}", e);
            std::process::exit(1);
        }
    };

    // 3. Subscribe to Vehicle.Command.Door.Lock.
    let mut stream = match client.subscribe_signal(COMMAND_SIGNAL).await {
        Ok(s) => s,
        Err(e) => {
            error!("Fatal: could not subscribe to {}: {}", COMMAND_SIGNAL, e);
            std::process::exit(1);
        }
    };

    info!("LOCKING_SERVICE started");

    // 4. Install signal handlers and enter command processing loop.
    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
    let sigint = tokio::signal::ctrl_c();
    tokio::pin!(sigint);

    loop {
        tokio::select! {
            // SIGTERM received.
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down...");
                break;
            }
            // SIGINT (Ctrl+C) received.
            _ = &mut sigint => {
                info!("Received SIGINT, shutting down...");
                break;
            }
            // Signal update from DATA_BROKER subscription.
            msg = stream.message() => {
                match msg {
                    Ok(Some(response)) => {
                        // Process each entry in the subscription response.
                        for (_path, datapoint) in response.entries {
                            if let Some(value) = datapoint.value {
                                if let Some(locking_service::databroker_client::kuksa::val::v2::value::TypedValue::String(json_str)) = value.typed_value {
                                    locking_service::process_command(&json_str, &mut client).await;
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // Stream ended — connection lost.
                        error!("DATA_BROKER subscription stream ended");
                        handle_reconnect(&mut client, &mut stream, &config.databroker_uds_path).await;
                    }
                    Err(e) => {
                        error!("DATA_BROKER subscription error: {}", e);
                        handle_reconnect(&mut client, &mut stream, &config.databroker_uds_path).await;
                    }
                }
            }
        }
    }

    // 5. Graceful shutdown: cancel subscriptions, close connection, exit 0.
    drop(stream);
    drop(client);
    info!("LOCKING_SERVICE stopped");
}

/// Handle DATA_BROKER reconnection with subscription restoration.
///
/// Detects broken streams, logs the error, and retries with exponential backoff
/// until the connection is re-established and subscriptions are restored.
async fn handle_reconnect(
    client: &mut DataBrokerClient,
    stream: &mut tonic::Streaming<locking_service::databroker_client::kuksa::val::v2::SubscribeResponse>,
    uds_path: &str,
) {
    loop {
        match client.reconnect(uds_path).await {
            Ok(()) => match client.subscribe_signal(COMMAND_SIGNAL).await {
                Ok(new_stream) => {
                    *stream = new_stream;
                    info!("Re-subscribed to {} after reconnection", COMMAND_SIGNAL);
                    return;
                }
                Err(e) => {
                    error!("Failed to re-subscribe after reconnection: {}", e);
                }
            },
            Err(e) => {
                error!("Reconnection attempt failed: {}", e);
            }
        }
    }
}
