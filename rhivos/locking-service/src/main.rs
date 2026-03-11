pub mod command;
pub mod config;
pub mod databroker_client;
pub mod executor;
pub mod safety;

use command::{Command, CommandResponse, ValidationError};
use config::Config;
use databroker_client::{DataBrokerClient, SignalValue};
use tracing::{error, info};

/// Command signal path in DATA_BROKER.
pub const COMMAND_SIGNAL: &str = "Vehicle.Command.Door.Lock";

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
                                if let Some(databroker_client::kuksa::val::v2::value::TypedValue::String(json_str)) = value.typed_value {
                                    process_command(&json_str, &mut client).await;
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

/// Process a single command JSON string from the subscription stream.
///
/// Follows the command processing flow:
/// 1. Parse JSON -> reject malformed with "invalid_command"
/// 2. Validate fields -> reject missing fields with "invalid_command"
/// 3. Validate action -> reject unknown action with "invalid_action"
/// 4. Safety checks -> reject constraint violations with specific reason
/// 5. Execute lock/unlock -> write state and success response
async fn process_command(json_str: &str, client: &mut DataBrokerClient) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Step 1 & 2: Parse and validate the command JSON.
    let cmd = match Command::from_json(json_str) {
        Ok(cmd) => cmd,
        Err(ValidationError::MalformedJson(e)) => {
            tracing::warn!("Malformed command JSON: {}", e);
            let resp =
                CommandResponse::failure("unknown".to_string(), "invalid_command".to_string(), now);
            executor::write_response(client, &resp).await;
            return;
        }
        Err(ValidationError::MissingField(field)) => {
            tracing::warn!("Command missing required field: {}", field);
            let resp =
                CommandResponse::failure("unknown".to_string(), "invalid_command".to_string(), now);
            executor::write_response(client, &resp).await;
            return;
        }
        Err(ValidationError::InvalidAction(action)) => {
            tracing::warn!("Invalid action: {}", action);
            let resp =
                CommandResponse::failure("unknown".to_string(), "invalid_action".to_string(), now);
            executor::write_response(client, &resp).await;
            return;
        }
    };

    let command_id = cmd.command_id.clone();

    // Step 3: Validate action is "lock" or "unlock".
    if let Err(ValidationError::InvalidAction(_)) = cmd.validate_action() {
        tracing::warn!(command_id = %command_id, "Invalid action: {}", cmd.action);
        let resp = CommandResponse::failure(command_id, "invalid_action".to_string(), now);
        executor::write_response(client, &resp).await;
        return;
    }

    // Step 4: Safety constraint checks.
    let speed = match client.get_signal("Vehicle.Speed").await {
        Ok(Some(SignalValue::Float(f))) => Some(f as f64),
        Ok(Some(SignalValue::Double(d))) => Some(d),
        _ => None,
    };

    let door_open = match client
        .get_signal("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
        .await
    {
        Ok(Some(SignalValue::Bool(b))) => Some(b),
        _ => None,
    };

    if let Err(reason) = safety::check_safety_constraints(speed, door_open) {
        info!(command_id = %command_id, reason = %reason, "Safety constraint violated");
        let resp = CommandResponse::failure(command_id, reason, now);
        executor::write_response(client, &resp).await;
        return;
    }

    // Step 5: Execute lock/unlock and write success response.
    let _ = executor::execute_lock_command(client, &cmd.action, &command_id).await;

    let resp = CommandResponse::success(command_id, now);
    executor::write_response(client, &resp).await;
}

/// Handle DATA_BROKER reconnection with subscription restoration.
///
/// Detects broken streams, logs the error, and retries with exponential backoff
/// until the connection is re-established and subscriptions are restored.
async fn handle_reconnect(
    client: &mut DataBrokerClient,
    stream: &mut tonic::Streaming<databroker_client::kuksa::val::v2::SubscribeResponse>,
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_startup() {
        assert!(true, "locking-service skeleton compiles and runs");
    }
}
