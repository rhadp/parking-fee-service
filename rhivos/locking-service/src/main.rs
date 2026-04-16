//! LOCKING_SERVICE — ASIL-B rated door lock control service.
//!
//! Subscribes to `Vehicle.Command.Door.Lock` from DATA_BROKER, validates
//! safety constraints (speed, door ajar), manages lock state, and publishes
//! responses to `Vehicle.Command.Door.Response`.
//!
//! Usage: locking-service serve

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

use tracing::{error, info, warn};

use broker::{BrokerClient, GrpcBrokerClient, SIGNAL_COMMAND_LOCK, SIGNAL_IS_LOCKED};
use command::{parse_command, validate_command, CommandError};
use config::get_databroker_addr;
use process::process_command;
use response::failure_response;

/// Service version string sourced from `Cargo.toml`.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum resubscription attempts before the service exits.
const MAX_RESUBSCRIBE_ATTEMPTS: u32 = 3;

fn print_usage() {
    eprintln!("Usage: locking-service serve");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Reject any flag-style arguments.
    for arg in &args[1..] {
        if arg.starts_with('-') {
            print_usage();
            std::process::exit(1);
        }
    }

    match args.get(1).map(|s| s.as_str()) {
        Some("serve") => {
            let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            let exit_code = runtime.block_on(run_service());
            std::process::exit(exit_code);
        }
        None => {
            // No subcommand: print version string and exit 0.
            println!("locking-service v{VERSION}");
        }
        Some(unknown) => {
            eprintln!("Unknown subcommand: {unknown}");
            print_usage();
            std::process::exit(1);
        }
    }
}

/// Run the service; returns the exit code (0 = clean, 1 = error).
async fn run_service() -> i32 {
    // Initialise structured logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let addr = get_databroker_addr();
    info!(version = VERSION, databroker_addr = %addr, "locking-service starting");

    // Connect to DATA_BROKER with exponential backoff (03-REQ-1.E1).
    let broker = match GrpcBrokerClient::connect(&addr).await {
        Ok(b) => b,
        Err(e) => {
            error!(error = ?e, "Failed to connect to DATA_BROKER; exiting");
            return 1;
        }
    };

    // Publish initial lock state: unlocked (03-REQ-4.3).
    if let Err(e) = broker.set_bool(SIGNAL_IS_LOCKED, false).await {
        error!(error = ?e, "Failed to publish initial lock state; exiting");
        return 1;
    }
    let mut lock_state = false;

    // Set up graceful shutdown signal watchers (03-REQ-6.1).
    let shutdown = setup_shutdown_watcher();

    // Subscribe loop: re-subscribe on stream interruption (03-REQ-1.E2).
    let exit_code = subscribe_and_process(&broker, &mut lock_state, shutdown).await;

    info!("locking-service stopped");
    exit_code
}

/// Set up a shared watch channel that fires on SIGTERM or SIGINT.
fn setup_shutdown_watcher() -> tokio::sync::watch::Receiver<bool> {
    let (tx, rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        // Wait for either SIGTERM or Ctrl-C.
        let shutdown_requested = wait_for_signal().await;
        if shutdown_requested {
            info!("Shutdown signal received; finishing current command then exiting");
            let _ = tx.send(true);
        }
    });
    rx
}

/// Wait for SIGTERM or SIGINT; returns true when a signal is received.
async fn wait_for_signal() -> bool {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(_) => {
                // If we can't register SIGTERM, fall back to ctrl_c only.
                tokio::signal::ctrl_c().await.ok();
                return true;
            }
        };
        tokio::select! {
            _ = sigterm.recv() => true,
            _ = tokio::signal::ctrl_c() => true,
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.ok();
        true
    }
}

/// Subscribe to command signal and process commands until shutdown or fatal error.
async fn subscribe_and_process(
    broker: &GrpcBrokerClient,
    lock_state: &mut bool,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> i32 {
    for attempt in 0..=MAX_RESUBSCRIBE_ATTEMPTS {
        if attempt > 0 {
            warn!(attempt, "Resubscribing to command signal");
        }

        let mut rx = match broker.subscribe(SIGNAL_COMMAND_LOCK).await {
            Ok(r) => r,
            Err(e) => {
                error!(error = ?e, attempt, "Failed to subscribe to command signal");
                if attempt < MAX_RESUBSCRIBE_ATTEMPTS {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
                error!("Exhausted subscription attempts; exiting");
                return 1;
            }
        };

        info!(signal = SIGNAL_COMMAND_LOCK, "Subscribed to command signal");
        if attempt == 0 {
            // Log readiness after first successful subscription and initial state publish.
            info!("locking-service ready");
        }

        // Process commands until stream ends or shutdown is requested (03-REQ-6.E1).
        loop {
            tokio::select! {
                biased;

                // Prioritise shutdown to allow clean exit (03-REQ-6.E1: finish current command first).
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        return 0;
                    }
                }

                maybe_payload = rx.recv() => {
                    match maybe_payload {
                        Some(payload) => {
                            handle_command_payload(broker, &payload, lock_state).await;
                        }
                        None => {
                            // Subscription stream closed; attempt resubscribe.
                            warn!("Command subscription stream closed; will resubscribe");
                            break;
                        }
                    }
                }
            }
        }

        if attempt >= MAX_RESUBSCRIBE_ATTEMPTS {
            error!("Exhausted resubscription attempts; exiting");
            return 1;
        }
    }

    0
}

/// Process a single raw JSON command payload.
///
/// Handles parse errors, validation errors, and command execution.
/// Invalid JSON is discarded without a response (03-REQ-2.E1).
async fn handle_command_payload(broker: &GrpcBrokerClient, payload: &str, lock_state: &mut bool) {
    // Parse the command (03-REQ-2.1).
    let cmd = match parse_command(payload) {
        Ok(cmd) => cmd,
        Err(CommandError::InvalidJson(reason)) => {
            warn!(reason, "Discarding non-JSON payload (no response published)");
            return;
        }
        Err(CommandError::InvalidCommand(reason)) => {
            // Valid JSON but bad fields: try to extract command_id for the response.
            warn!(reason, "Command parse failed; sending invalid_command response");
            if let Some(id) = extract_command_id(payload) {
                let resp = failure_response(&id, "invalid_command");
                if let Err(e) = broker
                    .set_string(broker::SIGNAL_RESPONSE, &resp)
                    .await
                {
                    error!(error = ?e, "Failed to publish invalid_command response");
                }
            }
            return;
        }
        Err(CommandError::UnsupportedDoor(reason)) => {
            // This branch is unlikely from parse_command but handled for completeness.
            warn!(reason, "Unsupported door in parse; sending unsupported_door response");
            if let Some(id) = extract_command_id(payload) {
                let resp = failure_response(&id, "unsupported_door");
                if let Err(e) = broker
                    .set_string(broker::SIGNAL_RESPONSE, &resp)
                    .await
                {
                    error!(error = ?e, "Failed to publish unsupported_door response");
                }
            }
            return;
        }
    };

    // Validate semantic constraints (03-REQ-2.2, 03-REQ-2.3).
    if let Err(e) = validate_command(&cmd) {
        let resp = failure_response(&cmd.command_id, e.reason());
        if let Err(publish_err) = broker.set_string(broker::SIGNAL_RESPONSE, &resp).await {
            error!(error = ?publish_err, "Failed to publish validation error response");
        }
        return;
    }

    // Execute the command.
    process_command(broker, &cmd, lock_state).await;
}

/// Try to extract `command_id` from a raw JSON string.
///
/// Returns `None` if the JSON is invalid or the field is absent.
fn extract_command_id(raw_json: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(raw_json)
        .ok()
        .and_then(|v| {
            v.get("command_id")
                .and_then(|id| id.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_owned())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_compiles() {
        assert!(true);
    }

    #[test]
    fn test_extract_command_id_present() {
        let json = r#"{"command_id":"abc-123","action":"lock","doors":["driver"]}"#;
        assert_eq!(extract_command_id(json), Some("abc-123".to_owned()));
    }

    #[test]
    fn test_extract_command_id_missing() {
        let json = r#"{"action":"lock","doors":["driver"]}"#;
        assert_eq!(extract_command_id(json), None);
    }

    #[test]
    fn test_extract_command_id_empty() {
        let json = r#"{"command_id":"","action":"lock","doors":["driver"]}"#;
        assert_eq!(extract_command_id(json), None);
    }

    #[test]
    fn test_extract_command_id_invalid_json() {
        assert_eq!(extract_command_id("not json {{{"), None);
    }
}
