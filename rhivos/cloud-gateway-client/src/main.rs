//! Entry point for CLOUD_GATEWAY_CLIENT.
//!
//! Implements the startup sequence per [04-REQ-9.1] / [04-REQ-9.2] in strict
//! order, then spawns three concurrent async tasks:
//!
//! 1. **command_loop** — receives NATS commands, authenticates them, validates
//!    the payload, and writes valid commands to DATA_BROKER.
//! 2. **response_loop** — relays command responses from DATA_BROKER to NATS
//!    verbatim.
//! 3. **telemetry_loop** — aggregates telemetry signal updates from DATA_BROKER
//!    and publishes the aggregated JSON to NATS on every change.

use std::process;
use std::sync::Arc;

use futures::StreamExt;
use tracing::{error, info, warn};

use cloud_gateway_client::broker_client::BrokerClient;
use cloud_gateway_client::command_validator::{validate_bearer_token, validate_command_payload};
use cloud_gateway_client::config::Config;
use cloud_gateway_client::errors::BrokerError;
use cloud_gateway_client::models::SignalUpdate;
use cloud_gateway_client::nats_client::NatsClient;
use cloud_gateway_client::telemetry::TelemetryState;

#[tokio::main]
async fn main() {
    // ── 01-REQ-4.E1: Reject unknown CLI flag arguments ──────────────────────
    // This service is configured entirely via environment variables.
    // Any argument starting with '-' is an unknown flag and must be rejected.
    for arg in std::env::args().skip(1) {
        if arg.starts_with('-') {
            eprintln!("Error: unknown flag '{arg}'; this service is configured via environment variables");
            process::exit(1);
        }
    }

    // ── Task 7.5: Initialize structured logging ─────────────────────────────
    // Respects RUST_LOG env var; defaults to INFO when unset or invalid.
    // [04-REQ-10.1]
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // ── REQ-9.1 Step 1: Read and validate environment variables ─────────────
    let config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Configuration error: {e}");
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    info!(vin = %config.vin, "cloud-gateway-client starting");

    // ── REQ-9.1 Step 2: Connect to NATS ────────────────────────────────────
    // [04-REQ-2.1], [04-REQ-2.2], [04-REQ-2.E1]
    let nats = match NatsClient::connect(&config).await {
        Ok(client) => Arc::new(client),
        Err(e) => {
            error!("Failed to connect to NATS: {e}");
            process::exit(1);
        }
    };

    // ── REQ-9.1 Step 3: Connect to DATA_BROKER ─────────────────────────────
    // [04-REQ-3.1], [04-REQ-3.E1]
    let broker = match BrokerClient::connect(&config).await {
        Ok(client) => Arc::new(client),
        Err(e) => {
            error!("Failed to connect to DATA_BROKER: {e}");
            process::exit(1);
        }
    };

    // ── REQ-9.1 Step 4: Publish self-registration ───────────────────────────
    // [04-REQ-4.1], [04-REQ-4.2]
    if let Err(e) = nats.publish_registration().await {
        error!("Failed to publish self-registration to NATS: {e}");
        process::exit(1);
    }

    // ── REQ-9.1 Step 5: Begin processing commands and telemetry ────────────
    // Subscribe to NATS commands subject. [04-REQ-2.3]
    let commands_sub = match nats.subscribe_commands().await {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to NATS commands: {e}");
            process::exit(1);
        }
    };

    // Subscribe to DATA_BROKER command response signal. [04-REQ-3.3]
    let response_stream = match broker.subscribe_responses().await {
        Ok(s) => Box::pin(s),
        Err(e) => {
            error!("Failed to subscribe to DATA_BROKER command responses: {e}");
            process::exit(1);
        }
    };

    // Subscribe to DATA_BROKER telemetry signals. [04-REQ-3.2]
    let telemetry_stream = match broker.subscribe_telemetry().await {
        Ok(s) => Box::pin(s),
        Err(e) => {
            error!("Failed to subscribe to DATA_BROKER telemetry signals: {e}");
            process::exit(1);
        }
    };

    info!(vin = %config.vin, "All subscriptions active; entering processing loops");

    // ── Task 7.2: Command processing loop ───────────────────────────────────
    let broker_cmd = Arc::clone(&broker);
    let bearer_token = config.bearer_token.clone();
    let cmd_task = tokio::spawn(async move {
        command_loop(commands_sub, broker_cmd, bearer_token).await;
    });

    // ── Task 7.3: Response relay loop ───────────────────────────────────────
    let nats_resp = Arc::clone(&nats);
    let resp_task = tokio::spawn(async move {
        response_loop(response_stream, nats_resp).await;
    });

    // ── Task 7.4: Telemetry loop ────────────────────────────────────────────
    let nats_telem = Arc::clone(&nats);
    let vin = config.vin.clone();
    let telem_task = tokio::spawn(async move {
        telemetry_loop(telemetry_stream, nats_telem, vin).await;
    });

    // The service runs indefinitely. Log if any loop exits unexpectedly.
    tokio::select! {
        result = cmd_task => {
            error!("Command processing loop exited unexpectedly: {:?}", result);
        }
        result = resp_task => {
            error!("Response relay loop exited unexpectedly: {:?}", result);
        }
        result = telem_task => {
            error!("Telemetry loop exited unexpectedly: {:?}", result);
        }
    }
}

/// Receive commands from NATS, authenticate them, validate their payload,
/// and forward valid commands verbatim to DATA_BROKER.
///
/// Discards messages that fail authentication or validation, logging at WARN.
///
/// Satisfies: [04-REQ-5.1], [04-REQ-5.2], [04-REQ-5.E1], [04-REQ-5.E2],
///            [04-REQ-6.1], [04-REQ-6.2], [04-REQ-6.3], [04-REQ-10.2],
///            [04-REQ-10.3]
async fn command_loop(
    mut subscriber: async_nats::Subscriber,
    broker: Arc<BrokerClient>,
    bearer_token: String,
) {
    while let Some(msg) = subscriber.next().await {
        // Step 1: Authenticate bearer token. [04-REQ-5.1], [04-REQ-5.2]
        let headers = match &msg.headers {
            Some(h) => h,
            None => {
                // [04-REQ-5.E1] Missing Authorization header → discard.
                warn!("Command discarded: NATS message has no headers (Authorization required)");
                continue;
            }
        };

        if let Err(e) = validate_bearer_token(headers, &bearer_token) {
            // [04-REQ-5.E1], [04-REQ-5.E2] → discard.
            warn!("Command discarded: bearer token authentication failed — {e}");
            continue;
        }

        // Step 2: Decode payload bytes as UTF-8.
        let payload_bytes = msg.payload.as_ref();
        let payload_str = match std::str::from_utf8(payload_bytes) {
            Ok(s) => s,
            Err(e) => {
                warn!("Command discarded: payload is not valid UTF-8 — {e}");
                continue;
            }
        };

        // Step 3: Validate command structure. [04-REQ-6.1], [04-REQ-6.2]
        let cmd = match validate_command_payload(payload_bytes) {
            Ok(c) => c,
            Err(e) => {
                // [04-REQ-6.E1], [04-REQ-6.E2], [04-REQ-6.E3] → discard.
                warn!("Command discarded: payload validation failed — {e}");
                continue;
            }
        };

        // Step 4: Forward the original payload verbatim to DATA_BROKER.
        // [04-REQ-6.3] Command Passthrough Fidelity (Prop 3).
        info!(
            command_id = %cmd.command_id,
            action = %cmd.action,
            "Command validated; forwarding to DATA_BROKER"
        );
        if let Err(e) = broker.write_command(payload_str).await {
            error!("Failed to write command to DATA_BROKER: {e}");
        }
    }

    warn!("NATS command subscription stream closed");
}

/// Relay command responses from DATA_BROKER to NATS verbatim.
///
/// Validates that each received value is JSON before publishing.
/// Non-JSON values are logged at ERROR and dropped (not published).
///
/// Satisfies: [04-REQ-7.1], [04-REQ-7.2], [04-REQ-7.E1], [04-REQ-10.2],
///            [04-REQ-10.4]
async fn response_loop<S>(mut stream: S, nats: Arc<NatsClient>)
where
    S: futures::Stream<Item = Result<String, BrokerError>> + Unpin,
{
    while let Some(result) = stream.next().await {
        match result {
            Ok(json) => {
                // [04-REQ-7.E1]: Non-JSON value from DATA_BROKER → log and skip.
                if serde_json::from_str::<serde_json::Value>(&json).is_err() {
                    error!(
                        "DATA_BROKER command response is not valid JSON; skipping NATS publish"
                    );
                    continue;
                }
                // [04-REQ-7.1]: Publish verbatim to NATS.
                if let Err(e) = nats.publish_response(&json).await {
                    error!("Failed to relay command response to NATS: {e}");
                }
            }
            Err(e) => {
                error!("DATA_BROKER command response stream error: {e}");
                break;
            }
        }
    }

    warn!("DATA_BROKER command response subscription stream closed");
}

/// Aggregate telemetry signal updates and publish to NATS on each change.
///
/// Maintains a `TelemetryState` that omits fields that have never been set.
/// Every signal update triggers a NATS publish of the aggregated JSON.
///
/// Satisfies: [04-REQ-8.1], [04-REQ-8.2], [04-REQ-8.3], [04-REQ-10.2],
///            [04-REQ-10.4]
async fn telemetry_loop<S>(mut stream: S, nats: Arc<NatsClient>, vin: String)
where
    S: futures::Stream<Item = Result<SignalUpdate, BrokerError>> + Unpin,
{
    let mut state = TelemetryState::new(vin);

    while let Some(result) = stream.next().await {
        match result {
            Ok(update) => {
                if let Some(json) = state.update(update) {
                    if let Err(e) = nats.publish_telemetry(&json).await {
                        error!("Failed to publish telemetry to NATS: {e}");
                    }
                }
            }
            Err(e) => {
                error!("DATA_BROKER telemetry stream error: {e}");
                break;
            }
        }
    }

    warn!("DATA_BROKER telemetry subscription stream closed");
}
