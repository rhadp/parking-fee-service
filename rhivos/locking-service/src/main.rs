pub mod broker;
pub mod command;
pub mod config;
pub mod process;
pub mod response;
pub mod safety;

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod proptest_cases;

use broker::{BrokerClient, SIGNAL_COMMAND, SIGNAL_IS_LOCKED};
use command::{parse_command, validate_command, CommandError};
use process::process_command;
use response::failure_response;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // No args or --help: print version to stdout, usage to stderr, exit 0.
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        println!("locking-service v{}", env!("CARGO_PKG_VERSION"));
        print_usage();
        std::process::exit(0);
    }

    // Require "serve" subcommand.
    if args[1] != "serve" {
        eprintln!("Unknown command: {}", args[1]);
        print_usage();
        std::process::exit(1);
    }

    // Initialise structured logging (03-REQ-6.2).
    tracing_subscriber::fmt::init();

    let addr = config::get_databroker_addr();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        databroker_addr = %addr,
        "locking-service starting"
    );

    // Connect to DATA_BROKER with exponential backoff (03-REQ-1.E1).
    let client = match broker::GrpcBrokerClient::connect(&addr).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("failed to connect to DATA_BROKER: {e}");
            std::process::exit(1);
        }
    };

    // Publish initial lock state as false (03-REQ-4.3).
    if let Err(e) = client.set_bool(SIGNAL_IS_LOCKED, false).await {
        tracing::error!("failed to publish initial lock state: {e}");
        std::process::exit(1);
    }

    // Subscribe to lock/unlock command signal (03-REQ-1.1).
    let mut rx = match client.subscribe(SIGNAL_COMMAND).await {
        Ok(rx) => rx,
        Err(e) => {
            tracing::error!("failed to subscribe to command signal: {e}");
            std::process::exit(1);
        }
    };

    tracing::info!("locking-service ready");

    let mut lock_state = false;

    // Command processing loop with graceful shutdown (03-REQ-1.3, 03-REQ-6.1).
    loop {
        tokio::select! {
            // Process commands sequentially as they arrive.
            payload = rx.recv() => {
                match payload {
                    Some(data) => {
                        handle_command_payload(&client, &data, &mut lock_state).await;
                    }
                    None => {
                        // Subscription channel closed; the background task exited
                        // after exhausting resubscribe attempts (03-REQ-1.E2).
                        tracing::error!("subscription channel closed, exiting");
                        std::process::exit(1);
                    }
                }
            }
            // Graceful shutdown on SIGTERM/SIGINT (03-REQ-6.1, 03-REQ-6.E1).
            // tokio::select! waits for the current branch to complete before
            // checking the shutdown signal, so any in-progress command finishes.
            _ = shutdown_signal() => {
                tracing::info!("shutdown signal received, exiting");
                break;
            }
        }
    }
}

/// Handle a single command payload from the subscription stream.
///
/// Parses, validates, and processes the command. Invalid JSON is discarded
/// without publishing a response (03-REQ-2.E1). Invalid commands with an
/// extractable command_id produce a failure response.
async fn handle_command_payload<B: BrokerClient>(
    broker: &B,
    payload: &str,
    lock_state: &mut bool,
) {
    tracing::info!("received command payload");

    // Parse the command JSON.
    let cmd = match parse_command(payload) {
        Ok(cmd) => cmd,
        Err(CommandError::InvalidJson(msg)) => {
            // Invalid JSON: log warning and discard without response (03-REQ-2.E1).
            tracing::warn!("discarding invalid JSON payload: {msg}");
            return;
        }
        Err(e) => {
            // Structural error: attempt to extract command_id for response.
            if let Some(cmd_id) = extract_command_id(payload) {
                let response = failure_response(&cmd_id, e.reason());
                if let Err(pub_err) =
                    broker.set_string(broker::SIGNAL_RESPONSE, &response).await
                {
                    tracing::error!("failed to publish error response: {pub_err}");
                }
            } else {
                tracing::warn!("invalid command with no extractable command_id: {e:?}");
            }
            return;
        }
    };

    // Validate business rules.
    if let Err(e) = validate_command(&cmd) {
        let response = failure_response(&cmd.command_id, e.reason());
        if let Err(pub_err) = broker.set_string(broker::SIGNAL_RESPONSE, &response).await {
            tracing::error!("failed to publish error response: {pub_err}");
        }
        return;
    }

    // Process the validated command.
    process_command(broker, &cmd, lock_state).await;
}

/// Attempt to extract a command_id from a partial JSON payload.
///
/// Used when serde deserialization fails but the payload might contain a
/// command_id field that can be used in the error response (03-REQ-2.E2).
fn extract_command_id(payload: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(payload).ok()?;
    let id = parsed.get("command_id")?.as_str()?;
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

/// Wait for a shutdown signal (SIGTERM or SIGINT).
async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {}
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
    }
}

/// Print usage information.
fn print_usage() {
    eprintln!("Usage: locking-service <command>");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  serve    Start the locking service");
    eprintln!();
    eprintln!("RHIVOS locking service for door lock/unlock commands.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_command_id_valid() {
        let payload = r#"{"command_id":"abc-123","action":"toggle","doors":["driver"]}"#;
        assert_eq!(extract_command_id(payload), Some("abc-123".to_string()));
    }

    #[test]
    fn test_extract_command_id_empty() {
        let payload = r#"{"command_id":"","action":"lock"}"#;
        assert_eq!(extract_command_id(payload), None);
    }

    #[test]
    fn test_extract_command_id_missing() {
        let payload = r#"{"action":"lock"}"#;
        assert_eq!(extract_command_id(payload), None);
    }

    #[test]
    fn test_extract_command_id_invalid_json() {
        assert_eq!(extract_command_id("not json"), None);
    }

    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
