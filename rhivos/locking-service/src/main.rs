//! LOCKING_SERVICE — ASIL-B rated Rust service (RHIVOS safety partition).
//!
//! Subscribes to `Vehicle.Command.Door.Lock` from DATA_BROKER, validates safety
//! constraints (vehicle speed, door ajar), manages the lock state, and publishes
//! command responses.

mod broker;
mod command;
mod config;
mod process;
mod response;
mod safety;

#[cfg(test)]
pub mod testing;

#[cfg(test)]
pub mod proptest_cases;

use tracing::{error, info, warn};

use broker::{BrokerClient, GrpcBrokerClient, SIGNAL_COMMAND, SIGNAL_IS_LOCKED};
use command::{parse_command, validate_command, CommandError};
use config::get_databroker_addr;
use process::process_command;
use response::failure_response;

/// Extract `command_id` from a partial JSON payload (best-effort).
///
/// Used to send an error response even when full parsing fails (design Path 5).
fn extract_command_id(json: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get("command_id").and_then(|s| s.as_str()).map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
}

/// Handle a single raw command payload.
///
/// - Invalid JSON: log warning, no response (03-REQ-2.E1).
/// - Parse/validate error: publish failure response if command_id is extractable.
/// - Valid command: dispatch to `process_command`.
async fn handle_command_payload(
    broker: &GrpcBrokerClient,
    payload: &str,
    lock_state: &mut bool,
) {
    match parse_command(payload) {
        Err(CommandError::InvalidJson(_)) => {
            warn!("Received invalid JSON payload — discarding without response");
        }
        Err(e) => {
            // Valid JSON but missing/invalid fields. Publish failure if we can
            // extract a command_id (03-REQ-2.E2, 03-REQ-2.E3).
            let reason = e.reason().to_string();
            if let Some(cid) = extract_command_id(payload) {
                let resp = failure_response(&cid, &reason);
                if let Err(pub_err) = broker.set_string(
                    crate::broker::SIGNAL_RESPONSE,
                    &resp,
                ).await {
                    error!("Failed to publish parse-error response: {pub_err}");
                }
            } else {
                warn!("Command parse error '{reason}' with no extractable command_id — discarding");
            }
        }
        Ok(cmd) => {
            // Semantic validation (empty command_id, unsupported door).
            if let Err(e) = validate_command(&cmd) {
                let reason = e.reason().to_string();
                let resp = failure_response(&cmd.command_id, &reason);
                if let Err(pub_err) = broker.set_string(
                    crate::broker::SIGNAL_RESPONSE,
                    &resp,
                ).await {
                    error!("Failed to publish validation-error response: {pub_err}");
                }
                return;
            }

            // Fully valid command — process it.
            process_command(broker, &cmd, lock_state).await;
        }
    }
}

/// Returns a future that resolves when SIGTERM is received (Unix only).
/// On non-Unix platforms, this future never resolves.
async fn wait_for_sigterm() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => {
                warn!("Failed to register SIGTERM handler: {e}");
                futures::future::pending::<()>().await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        futures::future::pending::<()>().await;
    }
}

/// Run the locking-service main loop.
///
/// 1. Connect to DATA_BROKER.
/// 2. Publish initial lock state (false).
/// 3. Subscribe to command signal.
/// 4. Process commands sequentially until shutdown or fatal error.
async fn run_service() {
    let addr = get_databroker_addr();
    info!(
        version = env!("CARGO_PKG_VERSION"),
        databroker_addr = %addr,
        "locking-service starting"
    );

    // Connect with exponential backoff (03-REQ-1.E1).
    let broker = match GrpcBrokerClient::connect(&addr).await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to connect to DATA_BROKER: {e}");
            std::process::exit(1);
        }
    };

    // Publish initial lock state = false (03-REQ-4.3).
    if let Err(e) = broker.set_bool(SIGNAL_IS_LOCKED, false).await {
        error!("Failed to publish initial lock state: {e}");
        std::process::exit(1);
    }
    info!("Published initial lock state: false");

    // Maximum resubscription attempts before giving up (03-REQ-1.E2).
    const MAX_RESUB_ATTEMPTS: u32 = 3;
    let mut resub_attempts = 0u32;
    let mut lock_state = false;

    // Unified shutdown signal: SIGINT or SIGTERM.
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = wait_for_sigterm() => {}
        }
        let _ = shutdown_tx.send(true);
    });

    info!("locking-service ready");

    'outer: loop {
        // Subscribe to the command signal.
        let mut command_rx = match broker.subscribe(SIGNAL_COMMAND).await {
            Ok(rx) => rx,
            Err(e) => {
                resub_attempts += 1;
                if resub_attempts >= MAX_RESUB_ATTEMPTS {
                    error!("Exhausted resubscription attempts — exiting");
                    std::process::exit(1);
                }
                warn!("resubscribing to {SIGNAL_COMMAND} (attempt {resub_attempts}/{MAX_RESUB_ATTEMPTS}): {e}");
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                continue 'outer;
            }
        };

        info!("Subscribed to {SIGNAL_COMMAND}");

        // Command processing loop.
        loop {
            tokio::select! {
                biased;
                maybe_payload = command_rx.recv() => {
                    match maybe_payload {
                        Some(payload) => {
                            info!("Received command payload");
                            handle_command_payload(&broker, &payload, &mut lock_state).await;
                            resub_attempts = 0; // Reset on successful processing.
                        }
                        None => {
                            // Subscription stream ended — attempt to resubscribe.
                            resub_attempts += 1;
                            if resub_attempts >= MAX_RESUB_ATTEMPTS {
                                error!("Subscription stream ended too many times — exiting");
                                std::process::exit(1);
                            }
                            warn!("resubscribing to {SIGNAL_COMMAND} (attempt {resub_attempts}/{MAX_RESUB_ATTEMPTS})");
                            break; // Break inner loop to resubscribe.
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    info!("Received shutdown signal — completing graceful shutdown");
                    break 'outer;
                }
            }
        }
    }

    info!("locking-service stopped");
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        println!("locking-service v{}", env!("CARGO_PKG_VERSION"));
        println!("Usage: locking-service serve");
        return;
    }

    let first = &args[0];

    if first == "--help" || first == "-h" {
        println!("locking-service v{}", env!("CARGO_PKG_VERSION"));
        println!("Usage: locking-service serve");
        return;
    }

    if first.starts_with('-') {
        eprintln!("Usage: locking-service serve");
        std::process::exit(1);
    }

    if first == "serve" {
        // Initialise structured logging.
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("locking_service=info".parse().unwrap()),
            )
            .init();

        run_service().await;
    } else {
        eprintln!("Unknown command: {first}");
        eprintln!("Usage: locking-service serve");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    /// Verifies the crate compiles successfully (01-REQ-8.1, TS-01-26).
    #[test]
    fn it_compiles() {
        assert!(true);
    }

    /// extract_command_id returns Some for valid JSON with non-empty command_id.
    #[test]
    fn test_extract_command_id_valid() {
        let json = r#"{"command_id":"abc-123","action":"lock"}"#;
        let id = super::extract_command_id(json);
        assert_eq!(id, Some("abc-123".to_string()));
    }

    /// extract_command_id returns None for empty command_id.
    #[test]
    fn test_extract_command_id_empty() {
        let json = r#"{"command_id":"","action":"lock"}"#;
        let id = super::extract_command_id(json);
        assert_eq!(id, None);
    }

    /// extract_command_id returns None for non-JSON input.
    #[test]
    fn test_extract_command_id_invalid_json() {
        let id = super::extract_command_id("not json");
        assert_eq!(id, None);
    }
}
