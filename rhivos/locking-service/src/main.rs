mod broker;
mod command;
mod config;
mod process;
mod response;
mod safety;

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod proptest_cases;

use broker::{BrokerClient, GrpcBrokerClient, SIGNAL_COMMAND, SIGNAL_IS_LOCKED, SIGNAL_RESPONSE};
use command::{parse_command, validate_command, CommandError};
use process::process_command;
use response::failure_response;

#[tokio::main]
async fn main() {
    // Parse CLI args: `serve` subcommand required
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("serve") => {}
        Some("--help") | Some("-h") | None => {
            println!("Usage: locking-service serve");
            println!("  RHIVOS locking service for door lock/unlock operations");
            std::process::exit(0);
        }
        Some(other) => {
            eprintln!("Unknown command: {other}");
            eprintln!("Usage: locking-service serve");
            std::process::exit(1);
        }
    }

    // Initialise structured logging (03-REQ-6.2)
    tracing_subscriber::fmt::init();

    let addr = config::get_databroker_addr();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        databroker_addr = %addr,
        "locking-service starting"
    );

    // Connect to DATA_BROKER with exponential backoff (03-REQ-1.E1)
    let client = match GrpcBrokerClient::connect(&addr).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to connect to DATA_BROKER");
            std::process::exit(1);
        }
    };

    // Publish initial lock state as unlocked (03-REQ-4.3)
    if let Err(e) = client.set_bool(SIGNAL_IS_LOCKED, false).await {
        tracing::error!(error = %e, "failed to publish initial lock state");
        std::process::exit(1);
    }

    // Subscribe to command signal (03-REQ-1.1)
    let mut rx = match client.subscribe(SIGNAL_COMMAND).await {
        Ok(rx) => rx,
        Err(e) => {
            tracing::error!(error = %e, "failed to subscribe to command signal");
            std::process::exit(1);
        }
    };

    tracing::info!("locking-service ready");

    let mut lock_state = false;

    // Register SIGTERM handler for graceful shutdown (03-REQ-6.1)
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("failed to register SIGTERM handler");

    // Command processing loop (03-REQ-1.3: commands processed sequentially)
    loop {
        tokio::select! {
            // Bias towards processing commands before checking shutdown signals,
            // ensuring the current command completes before exit (03-REQ-6.E1).
            biased;

            payload = rx.recv() => {
                match payload {
                    Some(json_str) => {
                        handle_command_payload(&client, &json_str, &mut lock_state).await;
                    }
                    None => {
                        // Subscription stream ended, attempt resubscription (03-REQ-1.E2)
                        tracing::warn!("subscription stream ended, attempting to resubscribe");
                        let mut resubscribed = false;
                        for attempt in 1..=5u32 {
                            tracing::warn!(attempt, "resubscribing to command signal");
                            match client.subscribe(SIGNAL_COMMAND).await {
                                Ok(new_rx) => {
                                    rx = new_rx;
                                    resubscribed = true;
                                    tracing::info!("resubscribed to command signal");
                                    break;
                                }
                                Err(e) => {
                                    tracing::error!(attempt, error = %e, "resubscription failed");
                                    let delay = 1u64 << (attempt - 1);
                                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                                }
                            }
                        }
                        if !resubscribed {
                            tracing::error!("failed to resubscribe after max attempts, exiting");
                            std::process::exit(1);
                        }
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("received SIGINT, shutting down");
                break;
            }
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM, shutting down");
                break;
            }
        }
    }

    tracing::info!("locking-service shutdown complete");
}

/// Handle a single command payload received from the subscription stream.
///
/// Parses, validates, and processes the command. Invalid JSON payloads are
/// discarded without response (03-REQ-2.E1). Invalid commands with an
/// extractable command_id receive a failure response (03-REQ-2.E2).
async fn handle_command_payload<B: BrokerClient>(
    broker: &B,
    payload: &str,
    lock_state: &mut bool,
) {
    tracing::info!("received command payload");

    // Parse command JSON
    let cmd = match parse_command(payload) {
        Ok(cmd) => cmd,
        Err(CommandError::InvalidJson(ref e)) => {
            // Invalid JSON: log warning, discard without response (03-REQ-2.E1)
            tracing::warn!(error = %e, "discarding invalid JSON payload");
            return;
        }
        Err(ref e) => {
            // Valid JSON but failed to parse as LockCommand (03-REQ-2.E2)
            if let Some(command_id) = extract_command_id(payload) {
                let response = failure_response(&command_id, e.reason());
                if let Err(pub_err) = broker.set_string(SIGNAL_RESPONSE, &response).await {
                    tracing::error!(error = %pub_err, "failed to publish error response");
                }
            } else {
                tracing::warn!("invalid command with no extractable command_id");
            }
            return;
        }
    };

    // Validate business rules
    if let Err(e) = validate_command(&cmd) {
        let response = failure_response(&cmd.command_id, e.reason());
        if let Err(pub_err) = broker.set_string(SIGNAL_RESPONSE, &response).await {
            tracing::error!(error = %pub_err, "failed to publish validation error response");
        }
        return;
    }

    // Process the validated command
    let _response = process_command(broker, &cmd, lock_state).await;
    tracing::info!(
        command_id = %cmd.command_id,
        action = ?cmd.action,
        lock_state = *lock_state,
        "command processed"
    );
}

/// Extract `command_id` from a JSON payload that failed full deserialization.
///
/// Returns `None` if the payload is not valid JSON, has no `command_id` field,
/// or the field is empty (03-REQ-2.E3).
fn extract_command_id(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let id = v.get("command_id")?.as_str()?;
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        // Verify the binary crate compiles successfully.
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
    }

    #[test]
    fn test_extract_command_id_valid() {
        let json = r#"{"command_id":"abc-123","unknown_field":true}"#;
        let id = super::extract_command_id(json);
        assert_eq!(id, Some("abc-123".to_string()));
    }

    #[test]
    fn test_extract_command_id_empty() {
        let json = r#"{"command_id":"","action":"lock"}"#;
        let id = super::extract_command_id(json);
        assert_eq!(id, None);
    }

    #[test]
    fn test_extract_command_id_missing() {
        let json = r#"{"action":"lock"}"#;
        let id = super::extract_command_id(json);
        assert_eq!(id, None);
    }

    #[test]
    fn test_extract_command_id_invalid_json() {
        let id = super::extract_command_id("not json {{{");
        assert_eq!(id, None);
    }
}
