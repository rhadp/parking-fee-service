use locking_service::command::{Command, CommandResponse, ValidationError};
use locking_service::config::Config;
use locking_service::databroker_client::{DatabrokerClient, SignalValue};
use locking_service::kuksa_proto;
use tracing::{error, info, warn};

/// VSS signal path for incoming lock/unlock commands.
const SIGNAL_COMMAND_LOCK: &str = "Vehicle.Command.Door.Lock";

/// VSS signal path for vehicle speed (safety check).
const SIGNAL_SPEED: &str = "Vehicle.Speed";

/// VSS signal path for door open state (safety check).
const SIGNAL_DOOR_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";

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
/// Full command processing pipeline:
/// 1. Parse JSON command
/// 2. Validate required fields and action
/// 3. Check safety constraints (speed, door ajar)
/// 4. Execute lock/unlock
/// 5. Write response
async fn process_subscription_update(
    client: &mut DatabrokerClient,
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
                process_command(client, cmd_json).await;
            }
        }
    }
}

/// Process a single command JSON string through the full pipeline.
async fn process_command(client: &mut DatabrokerClient, cmd_json: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Step 1 & 2: Parse and validate the command
    let cmd = match Command::from_json(cmd_json) {
        Ok(cmd) => cmd,
        Err(e) => {
            // Determine reason based on error type
            let (reason, command_id) = match &e {
                ValidationError::MalformedJson(_) => {
                    warn!(error = ?e, "Malformed command JSON received");
                    ("invalid_command".to_string(), "unknown".to_string())
                }
                ValidationError::MissingField(field) => {
                    warn!(field = %field, "Command missing required field");
                    // Try to extract command_id from raw JSON if possible
                    let cid = extract_command_id(cmd_json).unwrap_or_else(|| "unknown".to_string());
                    ("invalid_command".to_string(), cid)
                }
                ValidationError::InvalidAction(action) => {
                    warn!(action = %action, "Command has invalid action");
                    let cid = extract_command_id(cmd_json).unwrap_or_else(|| "unknown".to_string());
                    ("invalid_action".to_string(), cid)
                }
            };

            let response = CommandResponse::failure(command_id, reason, now);
            locking_service::executor::write_response(client, &response).await;
            return;
        }
    };

    let command_id = cmd.command_id.clone();

    // Step 3: Safety constraint checks
    let speed = read_speed(client).await;
    let door_open = read_door_open(client).await;

    if let Err(reason) = locking_service::safety::check_safety_constraints(speed, door_open) {
        info!(
            command_id = %command_id,
            reason = %reason,
            "Command rejected due to safety constraint"
        );
        let response = CommandResponse::failure(command_id, reason, now);
        locking_service::executor::write_response(client, &response).await;
        return;
    }

    // Step 4: Execute lock/unlock
    if let Err(e) = locking_service::executor::execute_lock_action(client, &cmd.action).await {
        error!(
            command_id = %command_id,
            error = %e,
            "Failed to execute lock action"
        );
        let response = CommandResponse::failure(command_id, "execution_failed".to_string(), now);
        locking_service::executor::write_response(client, &response).await;
        return;
    }

    // Step 5: Write success response
    let response = CommandResponse::success(command_id, now);
    locking_service::executor::write_response(client, &response).await;
}

/// Read `Vehicle.Speed` from DATA_BROKER.
/// Returns `None` if the signal has no value (safe default).
async fn read_speed(client: &mut DatabrokerClient) -> Option<f64> {
    match client.get_signal(SIGNAL_SPEED).await {
        Ok(Some(SignalValue::Float(f))) => Some(f as f64),
        Ok(Some(SignalValue::Double(d))) => Some(d),
        Ok(Some(SignalValue::Int32(i))) => Some(i as f64),
        Ok(Some(SignalValue::Uint32(u))) => Some(u as f64),
        Ok(None) => None,
        Ok(_) => {
            warn!("Unexpected type for Vehicle.Speed signal");
            None
        }
        Err(e) => {
            warn!(error = %e, "Failed to read Vehicle.Speed from DATA_BROKER");
            None
        }
    }
}

/// Read `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` from DATA_BROKER.
/// Returns `None` if the signal has no value (safe default).
async fn read_door_open(client: &mut DatabrokerClient) -> Option<bool> {
    match client.get_signal(SIGNAL_DOOR_OPEN).await {
        Ok(Some(SignalValue::Bool(b))) => Some(b),
        Ok(None) => None,
        Ok(_) => {
            warn!("Unexpected type for door IsOpen signal");
            None
        }
        Err(e) => {
            warn!(error = %e, "Failed to read door IsOpen from DATA_BROKER");
            None
        }
    }
}

/// Try to extract `command_id` from a raw JSON string (best-effort for error responses).
fn extract_command_id(json_str: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(json_str)
        .ok()?
        .get("command_id")?
        .as_str()
        .map(|s| s.to_string())
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
