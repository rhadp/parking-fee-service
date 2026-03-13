// Include kuksa.val.v1 generated proto types (produced by build.rs / tonic-build).
pub mod kuksav1 {
    tonic::include_proto!("kuksa.val.v1");
}

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

use tracing::{error, info, warn};

use broker::{BrokerClient as _, GrpcBrokerClient};
use command::{parse_command, validate_command, CommandError};
use config::get_databroker_addr;
use process::{process_command, SIGNAL_IS_LOCKED, SIGNAL_RESPONSE};
use response::failure_response;

/// VSS path for the incoming lock/unlock command signal.
const SIGNAL_CMD: &str = "Vehicle.Command.Door.Lock";

/// Service version from Cargo.toml.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum subscription reconnect attempts (03-REQ-1.E2).
const MAX_RESUBSCRIBE_ATTEMPTS: u32 = 3;

#[tokio::main]
async fn main() {
    // Initialise structured logging.
    tracing_subscriber::fmt::init();

    let addr = get_databroker_addr();

    // Log startup metadata (03-REQ-6.2).
    info!(
        version = VERSION,
        databroker_addr = %addr,
        "locking-service starting"
    );

    // Connect to DATA_BROKER with exponential-backoff retry (03-REQ-1.E1).
    let broker = match GrpcBrokerClient::connect(&addr).await {
        Ok(b) => b,
        Err(e) => {
            error!(error = %e, "Failed to connect to DATA_BROKER — exiting");
            std::process::exit(1);
        }
    };

    // Publish initial lock state: unlocked (03-REQ-4.3).
    if let Err(e) = broker.set_bool(SIGNAL_IS_LOCKED, false).await {
        error!(error = %e, "Failed to publish initial lock state — exiting");
        std::process::exit(1);
    }

    // Internal lock state.  Starts false (unlocked) to match initial published state.
    let mut lock_state = false;

    info!("locking-service ready");

    // Shutdown watch: main sends `true` on SIGTERM/SIGINT.
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

    // Spawn a task that waits for SIGTERM/SIGINT and then signals shutdown.
    tokio::spawn(async move {
        wait_for_shutdown_signal().await;
        info!("Shutdown signal received — initiating graceful shutdown");
        let _ = shutdown_tx.send(true);
    });

    // ── Command processing loop with subscription retry ─────────────────────
    let mut resubscribe_failures: u32 = 0;

    'outer: loop {
        // Subscribe to the command signal (03-REQ-1.1, 03-REQ-1.E2).
        let mut cmd_rx = match broker.subscribe(SIGNAL_CMD).await {
            Ok(rx) => {
                resubscribe_failures = 0;
                rx
            }
            Err(e) => {
                resubscribe_failures += 1;
                if resubscribe_failures > MAX_RESUBSCRIBE_ATTEMPTS {
                    error!(
                        error = %e,
                        "Failed to subscribe after {} attempts — exiting",
                        MAX_RESUBSCRIBE_ATTEMPTS
                    );
                    std::process::exit(1);
                }
                warn!(
                    attempt = resubscribe_failures,
                    max = MAX_RESUBSCRIBE_ATTEMPTS,
                    error = %e,
                    "Subscribe failed, retrying"
                );
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        // Inner loop: process commands until the stream is interrupted.
        loop {
            tokio::select! {
                biased;
                // Check for shutdown before reading the next command.
                _ = shutdown_rx.changed() => {
                    info!("Command loop: shutdown signal received");
                    break 'outer;
                }
                // Receive the next command payload from the subscription.
                msg = cmd_rx.recv() => {
                    match msg {
                        // Channel closed: stream ended — try to resubscribe (03-REQ-1.E2).
                        None => {
                            resubscribe_failures += 1;
                            if resubscribe_failures > MAX_RESUBSCRIBE_ATTEMPTS {
                                error!(
                                    "Subscription stream interrupted, max resubscribe \
                                     attempts ({}) exceeded — exiting",
                                    MAX_RESUBSCRIBE_ATTEMPTS
                                );
                                std::process::exit(1);
                            }
                            warn!(
                                attempt = resubscribe_failures,
                                max = MAX_RESUBSCRIBE_ATTEMPTS,
                                "Subscription stream interrupted, resubscribing"
                            );
                            break; // break inner loop → outer loop retries subscribe
                        }
                        // A new command payload arrived (03-REQ-1.2).
                        Some(json) => {
                            handle_command_payload(
                                &broker,
                                &json,
                                &mut lock_state,
                            ).await;
                        }
                    }
                }
            }
        }
    }

    info!("locking-service shutdown complete");
    // Exit with code 0 (03-REQ-6.1).
    std::process::exit(0);
}

/// Parse, validate, and process a single command payload.
///
/// Routing:
/// - Invalid JSON          → log + discard (03-REQ-2.E1)
/// - Valid JSON but invalid field → failure response (03-REQ-2.E2, 03-REQ-2.E3)
/// - Valid command         → process_command (safety + state + response)
async fn handle_command_payload(
    broker: &GrpcBrokerClient,
    json: &str,
    lock_state: &mut bool,
) {
    match parse_command(json) {
        Err(CommandError::InvalidJson(e)) => {
            // Discard silently (no response) per 03-REQ-2.E1.
            warn!(error = %e, "Received invalid JSON payload — discarding");
        }
        Err(CommandError::InvalidCommand(e) | CommandError::UnsupportedDoor(e)) => {
            // serde deserialization failed on a valid JSON value (e.g. missing action).
            // Try to extract command_id from the raw JSON for the response.
            warn!(error = %e, "Command deserialization failed");
            if let Some(cmd_id) = extract_command_id(json) {
                let resp = failure_response(&cmd_id, "invalid_command");
                publish_response(broker, &resp).await;
            }
            // If we can't get a command_id we cannot produce a meaningful response.
        }
        Ok(cmd) => {
            // Further validation: empty command_id, unsupported door, etc.
            match validate_command(&cmd) {
                Err(e) => {
                    let resp = failure_response(&cmd.command_id, e.reason());
                    publish_response(broker, &resp).await;
                }
                Ok(()) => {
                    // Safety + state + response publishing (process_command handles all).
                    process_command(broker, &cmd, lock_state).await;
                }
            }
        }
    }
}

/// Try to extract `command_id` from a raw JSON string without full deserialization.
fn extract_command_id(json: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get("command_id").and_then(|id| id.as_str()).map(String::from))
        .filter(|s| !s.is_empty())
}

/// Publish a response string to DATA_BROKER, logging on failure (03-REQ-5.E1).
async fn publish_response(broker: &GrpcBrokerClient, response_json: &str) {
    if let Err(e) = broker.set_string(SIGNAL_RESPONSE, response_json).await {
        error!(error = %e, "Failed to publish command response");
    }
}

/// Block until SIGTERM or SIGINT is received (03-REQ-6.1).
async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM");
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received SIGINT");
            }
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to wait for Ctrl-C");
        info!("Received SIGINT");
    }
}
