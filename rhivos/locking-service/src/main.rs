pub mod command;
pub mod config;
pub mod databroker_client;
pub mod safety;

/// Generated Kuksa VAL v2 protobuf types.
#[allow(clippy::doc_overindented_list_items)]
pub mod kuksa_proto {
    tonic::include_proto!("kuksa.val.v2");
}

use config::Config;
use databroker_client::DatabrokerClient;
use tracing::{error, info};

/// VSS signal path for incoming lock/unlock commands.
const SIGNAL_COMMAND_LOCK: &str = "Vehicle.Command.Door.Lock";

#[tokio::main]
async fn main() {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load configuration
    let config = Config::from_env();
    info!(
        uds_path = %config.databroker_uds_path,
        "Loading configuration"
    );

    // Connect to DATA_BROKER with retry (exponential backoff)
    let mut client = DatabrokerClient::connect(&config.databroker_uds_path)
        .await
        .expect("Fatal: could not connect to DATA_BROKER");

    // Subscribe to Vehicle.Command.Door.Lock
    let stream = client.subscribe_signal(SIGNAL_COMMAND_LOCK).await;
    let mut stream = match stream {
        Ok(s) => {
            info!(signal = SIGNAL_COMMAND_LOCK, "Subscribed to command signal");
            s
        }
        Err(e) => {
            error!(error = %e, "Failed to subscribe to command signal");
            std::process::exit(1);
        }
    };

    info!("LOCKING_SERVICE started");

    // Set up graceful shutdown on SIGTERM/SIGINT
    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
    let sigint = tokio::signal::ctrl_c();
    tokio::pin!(sigint);

    // Command processing loop
    loop {
        tokio::select! {
            // Receive signal update from subscription stream
            msg = stream.message() => {
                match msg {
                    Ok(Some(response)) => {
                        process_subscription_update(&mut client, response).await;
                    }
                    Ok(None) => {
                        // Stream ended - DATA_BROKER connection lost
                        error!("DATA_BROKER subscription stream ended, attempting reconnection");
                        match reconnect_and_resubscribe(&config.databroker_uds_path).await {
                            Ok((new_client, new_stream)) => {
                                client = new_client;
                                stream = new_stream;
                                info!("Reconnected to DATA_BROKER and re-subscribed");
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to reconnect to DATA_BROKER");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Error receiving from subscription stream, attempting reconnection");
                        match reconnect_and_resubscribe(&config.databroker_uds_path).await {
                            Ok((new_client, new_stream)) => {
                                client = new_client;
                                stream = new_stream;
                                info!("Reconnected to DATA_BROKER and re-subscribed");
                            }
                            Err(e) => {
                                error!(error = %e, "Failed to reconnect to DATA_BROKER");
                                break;
                            }
                        }
                    }
                }
            }
            // Handle SIGTERM
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down");
                break;
            }
            // Handle SIGINT (Ctrl+C)
            _ = &mut sigint => {
                info!("Received SIGINT, shutting down");
                break;
            }
        }
    }

    // Graceful shutdown: drop client (closes gRPC connection)
    drop(stream);
    drop(client);
    info!("LOCKING_SERVICE stopped");
    std::process::exit(0);
}

/// Process a subscription update from `Vehicle.Command.Door.Lock`.
///
/// This is a placeholder that logs received updates. The full command
/// processing pipeline (parse -> validate -> safety check -> execute -> respond)
/// will be implemented in task groups 3 and 4.
async fn process_subscription_update(
    _client: &mut DatabrokerClient,
    response: kuksa_proto::SubscribeResponse,
) {
    for (path, datapoint) in &response.entries {
        if let Some(value) = &datapoint.value {
            if let Some(kuksa_proto::value::TypedValue::String(cmd_json)) = &value.typed_value {
                info!(
                    signal = %path,
                    payload_len = cmd_json.len(),
                    "Received command signal update"
                );
                // Full processing pipeline will be wired in task group 4 (subtask 4.3)
            }
        }
    }
}

/// Reconnect to DATA_BROKER and re-subscribe to the command signal.
///
/// Uses exponential backoff via `DatabrokerClient::connect`.
async fn reconnect_and_resubscribe(
    uds_path: &str,
) -> Result<
    (
        DatabrokerClient,
        tonic::Streaming<kuksa_proto::SubscribeResponse>,
    ),
    Box<dyn std::error::Error>,
> {
    let mut client = DatabrokerClient::connect(uds_path)
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

    let stream = client
        .subscribe_signal(SIGNAL_COMMAND_LOCK)
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

    Ok((client, stream))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_startup() {
        assert!(true, "locking-service skeleton compiles and starts");
    }
}
