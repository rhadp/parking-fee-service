pub mod broker;
pub mod command;
pub mod config;
pub mod process;
pub mod response;
pub mod safety;

#[cfg(test)]
pub mod proptest_cases;
#[cfg(test)]
pub mod testing;

use broker::{BrokerClient, GrpcBrokerClient};
use command::{parse_command, validate_command, CommandError};
use config::get_databroker_addr;
use process::process_command;
use response::failure_response;
use tracing::{error, info, warn};

/// Signal path for incoming lock/unlock commands.
const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
/// Signal path for the lock state.
const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
/// Signal path for command responses.
const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Service version string.
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();

    // Parse CLI: require `serve` subcommand
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        println!("locking-service v{}", VERSION);
        println!("Usage: locking-service serve");
        std::process::exit(0);
    }

    if args[1] != "serve" {
        eprintln!("Usage: locking-service serve");
        eprintln!("Error: unrecognized subcommand: {}", args[1]);
        std::process::exit(1);
    }

    let addr = get_databroker_addr();
    info!("locking-service v{} starting", VERSION);
    info!("connecting to DATA_BROKER at {}", addr);

    // Connect to DATA_BROKER with exponential backoff (03-REQ-1.E1)
    let mut client = match GrpcBrokerClient::connect(&addr).await {
        Ok(c) => c,
        Err(e) => {
            error!("failed to connect to DATA_BROKER: {}", e);
            std::process::exit(1);
        }
    };

    // Publish initial lock state as false (03-REQ-4.3)
    if let Err(e) = client.set_bool(SIGNAL_IS_LOCKED, false).await {
        error!("failed to publish initial lock state: {}", e);
        std::process::exit(1);
    }
    info!("published initial lock state: unlocked");

    // Subscribe to command signal (03-REQ-1.1)
    let mut command_rx = match client.subscribe(SIGNAL_COMMAND).await {
        Ok(rx) => rx,
        Err(e) => {
            error!("failed to subscribe to {}: {}", SIGNAL_COMMAND, e);
            std::process::exit(1);
        }
    };

    info!("locking-service ready");

    // Track current lock state
    let mut lock_state = false;

    // Set up graceful shutdown on SIGTERM/SIGINT (03-REQ-6.1)
    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to register SIGTERM handler");
    let sigint = tokio::signal::ctrl_c();
    tokio::pin!(sigint);

    // Command processing loop (03-REQ-1.3: sequential processing)
    loop {
        tokio::select! {
            // Bias towards completing current command before checking shutdown
            biased;

            payload = command_rx.recv() => {
                match payload {
                    Some(raw) => {
                        handle_command_payload(&client, &raw, &mut lock_state).await;
                    }
                    None => {
                        // Subscription stream ended — attempt resubscribe (03-REQ-1.E2)
                        warn!("subscription stream ended, resubscribing...");
                        match client.subscribe(SIGNAL_COMMAND).await {
                            Ok(rx) => {
                                command_rx = rx;
                                info!("resubscribed to {}", SIGNAL_COMMAND);
                            }
                            Err(e) => {
                                error!("failed to resubscribe: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                }
            }
            _ = sigterm.recv() => {
                info!("received SIGTERM, shutting down gracefully");
                break;
            }
            _ = &mut sigint => {
                info!("received SIGINT, shutting down gracefully");
                break;
            }
        }
    }

    info!("locking-service stopped");
}

/// Handle a single command payload from the subscription stream.
async fn handle_command_payload<B: BrokerClient>(
    broker: &B,
    raw: &str,
    lock_state: &mut bool,
) {
    info!("received command payload");

    // Parse JSON (03-REQ-2.E1: invalid JSON discarded without response)
    let cmd = match parse_command(raw) {
        Ok(cmd) => cmd,
        Err(CommandError::InvalidJson(e)) => {
            warn!("discarding invalid JSON payload: {}", e);
            return;
        }
        Err(CommandError::InvalidCommand(e)) => {
            warn!("invalid command: {}", e);
            if let Some(id) = extract_command_id(raw) {
                let resp = failure_response(&id, "invalid_command");
                if let Err(e) = broker.set_string(SIGNAL_RESPONSE, &resp).await {
                    error!("failed to publish error response: {}", e);
                }
            }
            return;
        }
        Err(CommandError::UnsupportedDoor(e)) => {
            warn!("unsupported door in parse: {}", e);
            if let Some(id) = extract_command_id(raw) {
                let resp = failure_response(&id, "unsupported_door");
                if let Err(e) = broker.set_string(SIGNAL_RESPONSE, &resp).await {
                    error!("failed to publish error response: {}", e);
                }
            }
            return;
        }
    };

    // Validate command fields (03-REQ-2.1, 03-REQ-2.2, 03-REQ-2.3)
    if let Err(err) = validate_command(&cmd) {
        warn!("command validation failed: {}", err.reason());
        let resp = failure_response(&cmd.command_id, err.reason());
        if let Err(e) = broker.set_string(SIGNAL_RESPONSE, &resp).await {
            error!("failed to publish error response: {}", e);
        }
        return;
    }

    // Process validated command
    let response = process_command(broker, &cmd, lock_state).await;
    info!("command {} processed: {}", cmd.command_id, &response);
}

/// Try to extract `command_id` from a raw JSON string that failed full parsing.
fn extract_command_id(raw: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    let id = v.get("command_id")?.as_str()?;
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_command_id_valid() {
        let raw = r#"{"command_id":"abc-123","unknown":"field"}"#;
        assert_eq!(extract_command_id(raw), Some("abc-123".to_string()));
    }

    #[test]
    fn test_extract_command_id_empty() {
        let raw = r#"{"command_id":"","action":"lock"}"#;
        assert_eq!(extract_command_id(raw), None);
    }

    #[test]
    fn test_extract_command_id_missing() {
        let raw = r#"{"action":"lock"}"#;
        assert_eq!(extract_command_id(raw), None);
    }

    #[test]
    fn test_extract_command_id_invalid_json() {
        assert_eq!(extract_command_id("not json"), None);
    }
}
