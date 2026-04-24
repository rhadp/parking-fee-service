//! CLOUD_GATEWAY_CLIENT entry point.
//!
//! Orchestrates the startup sequence and spawns async tasks for
//! command processing, response relay, and telemetry publishing.
//!
//! # Startup Sequence ([04-REQ-9.1])
//!
//! 1. Initialize tracing-subscriber for structured logging.
//! 2. Read and validate environment variables via `Config::from_env()`.
//! 3. Connect to NATS (with exponential backoff retry).
//! 4. Connect to DATA_BROKER via gRPC.
//! 5. Publish self-registration message to NATS.
//! 6. Spawn command processing, response relay, and telemetry tasks.
//!
//! A failure at any step logs the error and exits with code 1
//! without proceeding to subsequent steps ([04-REQ-9.2]).

use std::process::ExitCode;
use std::sync::Arc;

use futures::StreamExt;

use cloud_gateway_client::broker_client::BrokerClient;
use cloud_gateway_client::command_validator;
use cloud_gateway_client::config::Config;
use cloud_gateway_client::nats_client::NatsClient;
use cloud_gateway_client::telemetry::TelemetryState;

/// Runs the command processing loop.
///
/// Receives messages from the NATS command subscription, validates
/// the bearer token and command payload, then writes valid commands
/// to DATA_BROKER.
///
/// # Requirements
///
/// - [04-REQ-5.1], [04-REQ-5.2]: Extract and validate Authorization header.
/// - [04-REQ-5.E1], [04-REQ-5.E2]: Discard messages with missing/invalid tokens.
/// - [04-REQ-6.1]–[04-REQ-6.4]: Validate and forward command payload.
/// - [04-REQ-6.E1]–[04-REQ-6.E3]: Discard invalid payloads.
/// - [04-REQ-10.2]: Log validated commands at INFO.
/// - [04-REQ-10.3]: Log auth/validation failures at WARN.
async fn command_loop(
    nats: Arc<NatsClient>,
    broker: Arc<BrokerClient>,
    bearer_token: String,
) {
    let mut subscriber = match nats.subscribe_commands().await {
        Ok(sub) => sub,
        Err(e) => {
            tracing::error!(error = ?e, "failed to subscribe to commands, command loop exiting");
            return;
        }
    };

    tracing::info!("command processing loop started");

    while let Some(message) = subscriber.next().await {
        // Step 1: Extract Authorization header from NATS message headers.
        let auth_header: Option<&str> = message
            .headers
            .as_ref()
            .and_then(|h: &async_nats::HeaderMap| h.get("Authorization"))
            .map(|v: &async_nats::HeaderValue| v.as_str());

        // Step 2: Validate bearer token ([04-REQ-5.1], [04-REQ-5.2]).
        if let Err(e) = command_validator::validate_bearer_token(auth_header, &bearer_token) {
            tracing::warn!(error = ?e, "command authentication failed, discarding message");
            continue;
        }

        // Step 3: Validate command payload structure ([04-REQ-6.1], [04-REQ-6.2]).
        let payload_bytes = &message.payload;
        if let Err(e) = command_validator::validate_command_payload(payload_bytes) {
            tracing::warn!(error = ?e, "command validation failed, discarding message");
            continue;
        }

        // Step 4: Write the command payload as-is to DATA_BROKER ([04-REQ-6.3]).
        // The original payload is forwarded verbatim (passthrough fidelity).
        let payload_str = match std::str::from_utf8(payload_bytes) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "command payload is not valid UTF-8, discarding");
                continue;
            }
        };

        tracing::info!("validated command, forwarding to DATA_BROKER");

        if let Err(e) = broker.write_command(payload_str).await {
            tracing::error!(error = ?e, "failed to write command to DATA_BROKER");
        }
    }

    tracing::warn!("command subscription ended, command loop exiting");
}

/// Runs the response relay loop.
///
/// Subscribes to `Vehicle.Command.Door.Response` changes in DATA_BROKER
/// and publishes each response JSON verbatim to NATS.
///
/// # Requirements
///
/// - [04-REQ-7.1]: Read response JSON and publish to NATS verbatim.
/// - [04-REQ-7.2]: Response contains command_id, status, timestamp.
/// - [04-REQ-7.E1]: Skip relay when response is not valid JSON (handled in broker_client).
/// - [04-REQ-10.2]: Log each relayed response at INFO.
/// - [04-REQ-10.4]: Log NATS publish failures at ERROR.
async fn response_relay_loop(
    nats: Arc<NatsClient>,
    broker: Arc<BrokerClient>,
) {
    let mut rx = match broker.subscribe_responses().await {
        Ok(rx) => rx,
        Err(e) => {
            tracing::error!(error = ?e, "failed to subscribe to command responses, relay loop exiting");
            return;
        }
    };

    tracing::info!("response relay loop started");

    while let Some(response_json) = rx.recv().await {
        tracing::info!("relaying command response to NATS");

        if let Err(e) = nats.publish_response(&response_json).await {
            tracing::error!(error = ?e, "failed to publish command response to NATS");
        }
    }

    tracing::warn!("response subscription ended, relay loop exiting");
}

/// Runs the telemetry publishing loop.
///
/// Subscribes to telemetry signal changes in DATA_BROKER, aggregates
/// them via `TelemetryState`, and publishes updated telemetry JSON
/// to NATS on each change.
///
/// # Requirements
///
/// - [04-REQ-8.1]: Publish aggregated telemetry on signal change.
/// - [04-REQ-8.2]: JSON format with vin, is_locked, latitude, longitude, parking_active, timestamp.
/// - [04-REQ-8.3]: Omit fields that have never been set.
/// - [04-REQ-10.2]: Log each telemetry publish at INFO.
/// - [04-REQ-10.4]: Log NATS publish failures at ERROR.
async fn telemetry_loop(
    nats: Arc<NatsClient>,
    broker: Arc<BrokerClient>,
    vin: String,
) {
    let mut rx = match broker.subscribe_telemetry().await {
        Ok(rx) => rx,
        Err(e) => {
            tracing::error!(error = ?e, "failed to subscribe to telemetry signals, telemetry loop exiting");
            return;
        }
    };

    let mut state = TelemetryState::new(vin);

    tracing::info!("telemetry publishing loop started");

    while let Some(signal_update) = rx.recv().await {
        if let Some(telemetry_json) = state.update(signal_update) {
            tracing::info!("publishing telemetry update to NATS");

            if let Err(e) = nats.publish_telemetry(&telemetry_json).await {
                tracing::error!(error = ?e, "failed to publish telemetry to NATS");
            }
        }
    }

    tracing::warn!("telemetry subscription ended, telemetry loop exiting");
}

/// Async entry point implementing the startup sequence.
///
/// Returns `Ok(())` on graceful shutdown, or `Err(())` if a startup
/// step fails (caller should exit with code 1).
///
/// # Requirements
///
/// - [04-REQ-9.1]: Deterministic startup order.
/// - [04-REQ-9.2]: Fail fast on any step failure.
async fn run() -> Result<(), ()> {
    // Step 1: Read and validate configuration ([04-REQ-9.1] step 1).
    let config = Config::from_env().map_err(|e| {
        tracing::error!(error = ?e, "configuration error");
        eprintln!("Error: VIN environment variable is required");
    })?;

    tracing::info!(vin = %config.vin, "configuration loaded");

    // Step 2: Connect to NATS with retry ([04-REQ-9.1] step 2).
    let nats = NatsClient::connect(&config).await.map_err(|e| {
        tracing::error!(error = ?e, "failed to establish NATS connection");
    })?;

    let nats = Arc::new(nats);

    // Step 3: Connect to DATA_BROKER ([04-REQ-9.1] step 3).
    let broker = BrokerClient::connect(&config).await.map_err(|e| {
        tracing::error!(error = ?e, "failed to establish DATA_BROKER connection");
    })?;

    let broker = Arc::new(broker);

    // Step 4: Publish self-registration ([04-REQ-9.1] step 4, [04-REQ-4.1]).
    nats.publish_registration().await.map_err(|e| {
        tracing::error!(error = ?e, "failed to publish self-registration");
    })?;

    // Step 5: Begin processing commands and telemetry ([04-REQ-9.1] step 5).
    tracing::info!("starting command processing, response relay, and telemetry tasks");

    let cmd_handle = tokio::spawn(command_loop(
        Arc::clone(&nats),
        Arc::clone(&broker),
        config.bearer_token.clone(),
    ));

    let relay_handle = tokio::spawn(response_relay_loop(
        Arc::clone(&nats),
        Arc::clone(&broker),
    ));

    let telemetry_handle = tokio::spawn(telemetry_loop(
        Arc::clone(&nats),
        Arc::clone(&broker),
        config.vin.clone(),
    ));

    // Wait for all tasks to complete. In normal operation these run
    // indefinitely; they exit when the underlying subscriptions end
    // or the process receives a shutdown signal.
    let _ = tokio::try_join!(cmd_handle, relay_handle, telemetry_handle);

    tracing::info!("cloud-gateway-client shutting down");
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize tracing-subscriber for structured logging ([04-REQ-10.1]).
    //
    // Uses `RUST_LOG` environment variable for filter configuration.
    // Defaults to INFO level if `RUST_LOG` is not set.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("cloud-gateway-client starting");

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::from(1),
    }
}
