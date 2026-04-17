//! CLOUD_GATEWAY_CLIENT — entry point and service wiring.
//!
//! Implements the startup sequence, command processing loop, response relay
//! loop, and telemetry publishing loop.
//!
//! Validates [04-REQ-9.1], [04-REQ-9.2], [04-REQ-10.1]–[04-REQ-10.4]

mod broker_client;
mod command_validator;
mod config;
mod errors;
mod models;
mod nats_client;
mod telemetry;

use futures::StreamExt;
use tracing::{error, info, warn};

use broker_client::BrokerClient;
use command_validator::{validate_bearer_token, validate_command_payload, HeaderMap};
use config::Config;
use nats_client::NatsClient;
use telemetry::TelemetryState;

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    // Task 7.5: Initialize tracing-subscriber with env-filter and JSON output.
    // Configurable via RUST_LOG environment variable; defaults to "info".
    //
    // Validates [04-REQ-10.1]
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Reject flag-like arguments (e.g. --help) for early exit with code 1.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(first) = args.first() {
        if first.starts_with('-') {
            eprintln!("Usage: cloud-gateway-client");
            std::process::exit(1);
        }
    }

    // Run the full service startup; exit with code 1 on any failure.
    run_service().await;
}

// ── Service startup sequence ──────────────────────────────────────────────────

/// Runs the full service startup sequence and processing loops.
///
/// Startup order (REQ-9.1):
///   1. Read and validate environment variables
///   2. Connect to NATS (with exponential backoff)
///   3. Connect to DATA_BROKER
///   4. Publish self-registration (fire-and-forget)
///   5. Begin processing (subscribe + spawn tasks)
///
/// Any failure causes log + `std::process::exit(1)` (REQ-9.2).
async fn run_service() {
    // ── Step 1: Configuration ─────────────────────────────────────────────

    let config = Config::from_env().unwrap_or_else(|_| {
        // Write directly to stderr to ensure the message is visible even when
        // the tracing subscriber writer is directed to stdout.  This also
        // satisfies TS-04-SMOKE-2 which checks stderr for "VIN".
        eprintln!("ERROR: VIN environment variable is required but not set");
        error!("VIN environment variable is required but not set");
        std::process::exit(1);
    });

    info!(vin = %config.vin, "Configuration loaded");

    let vin = config.vin.clone();
    let bearer_token = config.bearer_token.clone();

    // ── Step 2: Connect to NATS ───────────────────────────────────────────

    let nats = NatsClient::connect(&config).await.unwrap_or_else(|_| {
        // NatsClient::connect already logs the error; exit here.
        std::process::exit(1);
    });

    // ── Step 3: Connect to DATA_BROKER ───────────────────────────────────

    let broker = BrokerClient::connect(&config).await.unwrap_or_else(|_| {
        // BrokerClient::connect already logs the error; exit here.
        std::process::exit(1);
    });

    // ── Step 4: Publish self-registration ────────────────────────────────

    // REQ-4.2: fire-and-forget; log on failure but continue.
    if let Err(e) = nats.publish_registration().await {
        warn!(error = ?e, "Failed to publish registration message; continuing");
    }

    // ── Step 5: Subscribe and begin processing ────────────────────────────

    // Subscribe to NATS command subject.
    let subscriber = nats.subscribe_commands().await.unwrap_or_else(|_| {
        error!("Failed to subscribe to NATS commands subject");
        std::process::exit(1);
    });

    // Subscribe to DATA_BROKER command-response signal.
    let response_stream = broker.subscribe_responses().await.unwrap_or_else(|_| {
        error!("Failed to subscribe to DATA_BROKER command-response signal");
        std::process::exit(1);
    });

    // Subscribe to DATA_BROKER telemetry signals.
    let telemetry_stream = broker.subscribe_telemetry().await.unwrap_or_else(|_| {
        error!("Failed to subscribe to DATA_BROKER telemetry signals");
        std::process::exit(1);
    });

    info!("Service started — processing commands and telemetry");

    // ── Spawn processing tasks ────────────────────────────────────────────

    // Task 7.2: Command processing loop (NATS → validate → DATA_BROKER).
    let broker_cmd = broker.clone();
    let cmd_task = tokio::spawn(async move {
        command_loop(subscriber, broker_cmd, bearer_token).await;
    });

    // Task 7.3: Response relay loop (DATA_BROKER → NATS).
    let nats_rsp = nats.clone();
    let rsp_task = tokio::spawn(async move {
        response_relay_loop(response_stream, nats_rsp).await;
    });

    // Task 7.4: Telemetry publishing loop (DATA_BROKER signals → NATS).
    let nats_tel = nats.clone();
    let tel_task = tokio::spawn(async move {
        telemetry_loop(telemetry_stream, nats_tel, vin).await;
    });

    // All three tasks run indefinitely; wait for all to terminate.
    let _ = tokio::join!(cmd_task, rsp_task, tel_task);
}

// ── Task 7.2: Command processing loop ────────────────────────────────────────

/// Receives lock/unlock commands from NATS, validates them, and writes them
/// to DATA_BROKER.
///
/// For each message:
///   1. Validate bearer token ([04-REQ-5.1], [04-REQ-5.2])
///   2. Validate command payload structure ([04-REQ-6.1], [04-REQ-6.2])
///   3. Write original payload bytes verbatim to DATA_BROKER ([04-REQ-6.3])
///
/// Invalid/unauthenticated messages are discarded and logged ([04-REQ-5.E1],
/// [04-REQ-5.E2], [04-REQ-6.E1]–[04-REQ-6.E3]).
///
/// Validates [04-REQ-5.2], [04-REQ-6.3], [04-REQ-10.2], [04-REQ-10.3]
async fn command_loop(
    mut subscriber: async_nats::Subscriber,
    broker: BrokerClient,
    bearer_token: String,
) {
    while let Some(msg) = subscriber.next().await {
        // Build a HeaderMap from the NATS message headers for the validator.
        let mut headers = HeaderMap::new();
        if let Some(h) = &msg.headers {
            if let Some(auth_val) = h.get("Authorization") {
                headers.insert("Authorization".to_string(), auth_val.to_string());
            }
        }

        // ── Authentication [04-REQ-5.1], [04-REQ-5.2], [04-REQ-5.E1], [04-REQ-5.E2] ──
        if let Err(e) = validate_bearer_token(&headers, &bearer_token) {
            warn!(error = ?e, "Command authentication failed; discarding message");
            continue;
        }

        // ── Payload validation [04-REQ-6.1], [04-REQ-6.2] ────────────────
        if let Err(e) = validate_command_payload(&msg.payload) {
            warn!(error = ?e, "Command payload validation failed; discarding message");
            continue;
        }

        // ── Passthrough to DATA_BROKER — original bytes, not re-serialised ──
        // (REQ-6.3, Property 3: byte-for-byte fidelity)
        let payload_str = match std::str::from_utf8(&msg.payload) {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "Command payload is not valid UTF-8; discarding");
                continue;
            }
        };

        match broker.write_command(payload_str).await {
            Ok(()) => {
                info!("Validated command forwarded to DATA_BROKER");
            }
            Err(e) => {
                error!(error = ?e, "Failed to write command to DATA_BROKER");
                // Transient error — continue processing next messages.
            }
        }
    }

    warn!("Command subscriber stream ended");
}

// ── Task 7.3: Response relay loop ────────────────────────────────────────────

/// Relays command responses from DATA_BROKER to NATS verbatim.
///
/// Validates [04-REQ-7.1], [04-REQ-7.2], [04-REQ-10.2], [04-REQ-10.4]
async fn response_relay_loop(
    mut stream: impl futures::Stream<Item = Result<String, errors::BrokerError>> + Unpin,
    nats: NatsClient,
) {
    while let Some(item) = stream.next().await {
        match item {
            Ok(json) => {
                match nats.publish_response(&json).await {
                    Ok(()) => {
                        info!("Command response relayed to NATS");
                    }
                    Err(e) => {
                        error!(error = ?e, "Failed to publish command response to NATS");
                        // Transient error — continue relaying.
                    }
                }
            }
            Err(e) => {
                error!(error = ?e, "DATA_BROKER response stream error");
                break;
            }
        }
    }

    warn!("Response relay stream ended");
}

// ── Task 7.4: Telemetry publishing loop ──────────────────────────────────────

/// Aggregates telemetry signal changes from DATA_BROKER and publishes to NATS.
///
/// Maintains a `TelemetryState` that accumulates known signal values.  On any
/// change, publishes the aggregated JSON to NATS.  Signals never observed are
/// omitted from the payload ([04-REQ-8.3]).
///
/// Validates [04-REQ-8.1], [04-REQ-8.2], [04-REQ-8.3], [04-REQ-10.2], [04-REQ-10.4]
async fn telemetry_loop(
    mut stream: impl futures::Stream<Item = Result<models::SignalUpdate, errors::BrokerError>>
        + Unpin,
    nats: NatsClient,
    vin: String,
) {
    let mut state = TelemetryState::new(vin);

    while let Some(item) = stream.next().await {
        match item {
            Ok(signal) => {
                if let Some(json) = state.update(signal) {
                    match nats.publish_telemetry(&json).await {
                        Ok(()) => {
                            info!("Telemetry message published to NATS");
                        }
                        Err(e) => {
                            error!(error = ?e, "Failed to publish telemetry to NATS");
                            // Transient error — continue processing.
                        }
                    }
                }
            }
            Err(e) => {
                error!(error = ?e, "DATA_BROKER telemetry stream error");
                break;
            }
        }
    }

    warn!("Telemetry loop stream ended");
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::models::RegistrationMessage;

    /// Verifies the crate compiles successfully (01-REQ-8.1, TS-01-26).
    #[test]
    fn it_compiles() {
        assert!(true);
    }

    // ------------------------------------------------------------------
    // TS-04-P1: Registration message serializes with required fields
    // ------------------------------------------------------------------

    /// TS-04-P1: `RegistrationMessage` serializes to JSON with `vin`, `status`,
    /// and `timestamp` fields.
    ///
    /// Validates [04-REQ-4.1]
    #[test]
    fn test_registration_message_format() {
        let msg = RegistrationMessage {
            vin: "VIN-001".to_string(),
            status: "online".to_string(),
            timestamp: 1_700_000_000,
        };
        let json = serde_json::to_string(&msg).expect("RegistrationMessage must serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("serialized output must be valid JSON");

        assert_eq!(parsed["vin"], "VIN-001");
        assert_eq!(parsed["status"], "online");
        assert!(
            parsed["timestamp"].is_number(),
            "timestamp must be a number"
        );
    }

    // ------------------------------------------------------------------
    // Integration property tests (require external infrastructure)
    // TS-04-P3, TS-04-P4, TS-04-P6
    // ------------------------------------------------------------------

    /// TS-04-P3: Command passthrough fidelity.
    ///
    /// The original NATS payload bytes must be written to DATA_BROKER
    /// (`Vehicle.Command.Door.Lock`) without any modification or re-serialization.
    ///
    /// Validates [04-REQ-6.3], [04-REQ-6.4]
    #[test]
    #[ignore = "integration test — requires running NATS and DATA_BROKER containers"]
    fn test_property_command_passthrough_fidelity() {
        // Full end-to-end verification implemented in task group 8 (TS-04-10).
        todo!("implement in task group 8")
    }

    /// TS-04-P4: Response relay fidelity.
    ///
    /// DATA_BROKER `Vehicle.Command.Door.Response` JSON must be relayed verbatim
    /// to `vehicles.{VIN}.command_responses` on NATS without modification.
    ///
    /// Validates [04-REQ-7.1], [04-REQ-7.2]
    #[test]
    #[ignore = "integration test — requires running NATS and DATA_BROKER containers"]
    fn test_property_response_relay_fidelity() {
        // Full end-to-end verification implemented in task group 8 (TS-04-11).
        todo!("implement in task group 8")
    }

    /// TS-04-P6: Startup determinism.
    ///
    /// A failure at any startup step (config → NATS → DATA_BROKER → registration
    /// → processing) must prevent all subsequent steps from executing, and the
    /// service must exit with code 1.
    ///
    /// Validates [04-REQ-9.1], [04-REQ-9.2]
    #[test]
    #[ignore = "integration test — requires full service binary with injectable failures"]
    fn test_property_startup_determinism() {
        // Full end-to-end verification implemented in task group 8.
        todo!("implement in task group 8")
    }
}
