//! LOCKING_SERVICE — ASIL-B rated lock/unlock command processor.
//!
//! Start with: locking-service serve
//! Connects to DATA_BROKER, validates safety constraints, manages door lock state.

pub mod broker;
pub mod command;
pub mod config;
pub mod process;
pub mod response;
pub mod safety;

#[cfg(test)]
pub mod testing;
#[cfg(test)]
pub mod proptest_cases;

use broker::{BrokerClient, GrpcBrokerClient, SIGNAL_COMMAND, SIGNAL_IS_LOCKED, SIGNAL_RESPONSE};
use command::{parse_command, validate_command, CommandError};
use response::failure_response;

/// Extract command_id from a raw JSON string without full deserialization.
/// Used when parse_command fails (e.g. missing required field) to still
/// publish an error response (03-REQ-2.E2, design Path 5).
///
/// Returns None if the JSON is malformed or command_id is missing/not a string.
pub fn extract_command_id(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let command_id = value.get("command_id")?.as_str()?;
    Some(command_id.to_string())
}

async fn handle_command_payload(broker: &impl BrokerClient, json: &str, lock_state: &mut bool) {
    match parse_command(json) {
        Err(CommandError::InvalidJson(e)) => {
            tracing::warn!("Received invalid JSON payload, ignoring: {e}");
        }
        Err(e) => {
            // Structurally invalid — try to extract command_id for error response.
            if let Some(id) = extract_command_id(json) {
                let resp = failure_response(&id, e.reason());
                if let Err(pub_err) = broker.set_string(SIGNAL_RESPONSE, &resp).await {
                    tracing::error!("Failed to publish error response: {pub_err}");
                }
            } else {
                tracing::warn!("Could not extract command_id from invalid payload, no response sent");
            }
        }
        Ok(cmd) => match validate_command(&cmd) {
            Err(e) => {
                let resp = failure_response(&cmd.command_id, e.reason());
                if let Err(pub_err) = broker.set_string(SIGNAL_RESPONSE, &resp).await {
                    tracing::error!("Failed to publish validation error response: {pub_err}");
                }
            }
            Ok(()) => {
                process::process_command(broker, &cmd, lock_state).await;
            }
        },
    }
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {}
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        ctrl_c.await.expect("failed to listen for Ctrl+C");
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Require "serve" subcommand; otherwise print usage and exit 0.
    if args.len() < 2 || args[1] != "serve" {
        if args.iter().any(|a| a.starts_with('-') && a != "--") {
            eprintln!("usage: locking-service serve");
            std::process::exit(0);
        }
        println!("usage: locking-service serve");
        std::process::exit(0);
    }

    // Initialize tracing with env-filter.
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let addr = config::get_databroker_addr();
    tracing::info!(
        "locking-service v{} connecting to {addr}",
        env!("CARGO_PKG_VERSION")
    );

    let mut broker = match GrpcBrokerClient::connect(&addr).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to connect to DATA_BROKER: {e}");
            std::process::exit(1);
        }
    };

    // Publish initial state: unlocked.
    if let Err(e) = broker.set_bool(SIGNAL_IS_LOCKED, false).await {
        tracing::error!("Failed to publish initial state: {e}");
    }

    // Subscribe to command signal.
    let mut receiver = match broker.subscribe(SIGNAL_COMMAND).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to subscribe: {e}");
            std::process::exit(1);
        }
    };

    tracing::info!("locking-service ready");

    let mut lock_state = false;

    loop {
        tokio::select! {
            msg = receiver.recv() => {
                match msg {
                    Some(json) => {
                        handle_command_payload(&broker, &json, &mut lock_state).await;
                    }
                    None => {
                        tracing::warn!("Subscription stream ended, shutting down");
                        break;
                    }
                }
            }
            _ = shutdown_signal() => {
                tracing::info!("Shutdown signal received, exiting");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // extract_command_id — valid JSON with command_id present
    #[test]
    fn test_extract_command_id_present() {
        let json = r#"{"command_id":"abc-123","doors":["driver"]}"#;
        let result = extract_command_id(json);
        assert_eq!(result, Some("abc-123".to_string()));
    }

    // extract_command_id — valid JSON without command_id
    #[test]
    fn test_extract_command_id_missing() {
        let json = r#"{"action":"lock","doors":["driver"]}"#;
        let result = extract_command_id(json);
        assert!(result.is_none(), "should return None when command_id absent");
    }

    // extract_command_id — invalid JSON
    #[test]
    fn test_extract_command_id_invalid_json() {
        let result = extract_command_id("not json {{{{");
        assert!(result.is_none(), "should return None for invalid JSON");
    }

    // extract_command_id — command_id is empty string
    #[test]
    fn test_extract_command_id_empty_string() {
        let json = r#"{"command_id":""}"#;
        let result = extract_command_id(json);
        // Empty command_id is technically extractable but invalid;
        // extract_command_id returns it and the caller decides what to do.
        // This behaviour documents that extract_command_id doesn't validate.
        // (Validation is done by validate_command separately.)
        // We allow either Some("") or None — the implementation decides.
        // Test just checks it doesn't panic.
        let _ = result;
    }

    // extract_command_id — command_id is non-string type
    #[test]
    fn test_extract_command_id_non_string() {
        let json = r#"{"command_id":42}"#;
        let result = extract_command_id(json);
        assert!(result.is_none(), "non-string command_id should return None");
    }
}
