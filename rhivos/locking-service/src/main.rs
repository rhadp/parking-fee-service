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

use broker::{BrokerClient, GrpcBrokerClient, SIGNAL_COMMAND, SIGNAL_IS_LOCKED};
use command::{parse_command, validate_command, CommandError};
use process::process_command;
use response::failure_response;

/// Extract a command_id from a JSON string that may be partially valid.
///
/// Used when `parse_command` fails but we still want to publish a failure
/// response containing the command_id if one can be found.
fn extract_command_id(json: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(json).ok()?;
    parsed
        .get("command_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Handle a single command payload from the subscription stream.
///
/// Parses, validates, and processes the command. Publishes appropriate
/// responses for valid and invalid commands. Invalid JSON is discarded
/// without a response (03-REQ-2.E1).
async fn handle_command_payload(
    broker: &GrpcBrokerClient,
    payload: &str,
    lock_state: &mut bool,
) {
    tracing::info!("received command payload");

    // Parse the command.
    let cmd = match parse_command(payload) {
        Ok(cmd) => cmd,
        Err(CommandError::InvalidJson(msg)) => {
            // Invalid JSON: log warning, discard without response (03-REQ-2.E1).
            tracing::warn!("discarding invalid JSON payload: {msg}");
            return;
        }
        Err(e) => {
            // Valid JSON but missing/invalid fields: try to extract command_id
            // to publish a failure response (03-REQ-2.E2).
            if let Some(command_id) = extract_command_id(payload) {
                let response = failure_response(&command_id, e.reason());
                if let Err(publish_err) = broker
                    .set_string(broker::SIGNAL_RESPONSE, &response)
                    .await
                {
                    tracing::error!("failed to publish error response: {publish_err}");
                }
            } else {
                tracing::warn!("invalid command without extractable command_id: {e}");
            }
            return;
        }
    };

    // Validate the command.
    if let Err(e) = validate_command(&cmd) {
        tracing::warn!(
            command_id = %cmd.command_id,
            "command validation failed: {e}"
        );
        let response = failure_response(&cmd.command_id, e.reason());
        if let Err(publish_err) = broker
            .set_string(broker::SIGNAL_RESPONSE, &response)
            .await
        {
            tracing::error!("failed to publish error response: {publish_err}");
        }
        return;
    }

    // Process the validated command.
    process_command(broker, &cmd, lock_state).await;
}

/// Run the main service loop: subscribe, process commands, handle shutdown.
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let addr = config::get_databroker_addr();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        databroker_addr = %addr,
        "locking-service starting"
    );

    // Connect to DATA_BROKER with exponential backoff (03-REQ-1.E1).
    let broker = GrpcBrokerClient::connect(&addr).await.map_err(|e| {
        tracing::error!("failed to connect to DATA_BROKER: {e}");
        e
    })?;

    // Publish initial lock state as false (unlocked) (03-REQ-4.3).
    broker.set_bool(SIGNAL_IS_LOCKED, false).await.map_err(|e| {
        tracing::error!("failed to publish initial lock state: {e}");
        e
    })?;
    tracing::info!("published initial lock state: unlocked");

    // Subscribe to command signal (03-REQ-1.1).
    let mut rx = broker.subscribe(SIGNAL_COMMAND).await.map_err(|e| {
        tracing::error!("failed to subscribe to command signal: {e}");
        e
    })?;

    tracing::info!("locking-service ready");

    // Command processing loop with graceful shutdown (03-REQ-6.1, 03-REQ-6.E1).
    let mut lock_state = false;
    let max_resubscribe_attempts: u32 = 3;
    let mut resubscribe_count: u32 = 0;

    loop {
        tokio::select! {
            // Bias towards processing commands before checking shutdown.
            biased;

            payload = rx.recv() => {
                match payload {
                    Some(cmd_json) => {
                        // Reset resubscribe counter on successful receive.
                        resubscribe_count = 0;
                        // Process commands sequentially (03-REQ-1.3).
                        handle_command_payload(&broker, &cmd_json, &mut lock_state).await;
                    }
                    None => {
                        // Subscription stream ended (03-REQ-1.E2).
                        resubscribe_count += 1;
                        if resubscribe_count > max_resubscribe_attempts {
                            tracing::error!(
                                "subscription stream ended; exhausted {max_resubscribe_attempts} resubscribe attempts"
                            );
                            return Err("subscription lost".into());
                        }
                        tracing::warn!(
                            attempt = resubscribe_count,
                            "subscription stream ended, resubscribing"
                        );
                        match broker.subscribe(SIGNAL_COMMAND).await {
                            Ok(new_rx) => {
                                rx = new_rx;
                                tracing::info!("resubscribed to command signal");
                            }
                            Err(e) => {
                                tracing::error!("resubscribe failed: {e}");
                                return Err(e.into());
                            }
                        }
                    }
                }
            }

            _ = tokio::signal::ctrl_c() => {
                tracing::info!("received shutdown signal, exiting gracefully");
                break;
            }
        }
    }

    tracing::info!("locking-service shut down");
    Ok(())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Parse CLI args: `serve` subcommand required (03-REQ-6.2).
    // No args or `--help` prints usage and exits 0.
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        println!("locking-service v{}", env!("CARGO_PKG_VERSION"));
        println!();
        println!("Usage: {} serve", args[0]);
        println!();
        println!("Commands:");
        println!("  serve    Start the locking service");
        return;
    }

    if args[1] != "serve" {
        eprintln!(
            "Error: unknown command '{}'. Use 'serve' to start the service.",
            args[1]
        );
        std::process::exit(1);
    }

    // Initialise structured logging via tracing (03-REQ-6.2).
    tracing_subscriber::fmt::init();

    // Run the service; exit non-zero on fatal errors.
    if let Err(e) = run().await {
        tracing::error!("locking-service exited with error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_compiles() {
        // Placeholder test: verifies the crate compiles.
    }

    #[test]
    fn test_extract_command_id_valid() {
        let json = r#"{"command_id":"abc-123","action":"invalid"}"#;
        assert_eq!(extract_command_id(json), Some("abc-123".to_string()));
    }

    #[test]
    fn test_extract_command_id_missing() {
        let json = r#"{"action":"lock"}"#;
        assert_eq!(extract_command_id(json), None);
    }

    #[test]
    fn test_extract_command_id_empty() {
        let json = r#"{"command_id":"","action":"lock"}"#;
        assert_eq!(extract_command_id(json), None);
    }

    #[test]
    fn test_extract_command_id_invalid_json() {
        assert_eq!(extract_command_id("not json"), None);
    }
}
