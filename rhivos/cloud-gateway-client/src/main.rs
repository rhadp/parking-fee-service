pub mod broker_client;
pub mod command_validator;
pub mod config;
pub mod errors;
pub mod models;
pub mod nats_client;
pub mod telemetry;

#[cfg(test)]
mod tests;

use config::Config;
use std::process;
use tracing::{error, info, warn};

/// Entry point for the cloud-gateway-client service.
///
/// Startup sequencing (per [04-REQ-9.1]):
///   1. Read and validate environment variables
///   2. Connect to NATS (with exponential backoff retry)
///   3. Connect to DATA_BROKER
///   4. Publish self-registration
///   5. Begin processing commands, responses, and telemetry
///
/// If any step fails, the service logs the failure and exits with code 1
/// without proceeding to subsequent steps (per [04-REQ-9.2]).
#[tokio::main]
async fn main() {
    // Task 7.5: Initialize structured logging via tracing-subscriber.
    // Respects RUST_LOG env var for level filtering (e.g., RUST_LOG=debug).
    // Implements: [04-REQ-10.1]
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Step 1: Read and validate configuration.
    let config = match Config::from_env() {
        Ok(cfg) => {
            info!(vin = %cfg.vin, "Configuration loaded");
            cfg
        }
        Err(e) => {
            error!("Configuration error: {}", e);
            process::exit(1);
        }
    };

    // Step 2: Connect to NATS (with exponential backoff retry).
    let nats_client = match nats_client::NatsClient::connect(&config).await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to connect to NATS: {:?}", e);
            process::exit(1);
        }
    };

    // Step 3: Connect to DATA_BROKER.
    let broker_client = match broker_client::BrokerClient::connect(&config).await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to connect to DATA_BROKER: {:?}", e);
            process::exit(1);
        }
    };

    // Step 4: Publish self-registration (fire-and-forget).
    if let Err(e) = nats_client.publish_registration().await {
        error!("Failed to publish self-registration: {:?}", e);
        process::exit(1);
    }

    info!("Startup complete, beginning processing");

    // Step 5: Spawn concurrent processing tasks.

    // Subscribe to NATS commands subject.
    let mut cmd_subscriber = match nats_client.subscribe_commands().await {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to commands: {:?}", e);
            process::exit(1);
        }
    };

    // Subscribe to DATA_BROKER command responses.
    let mut response_rx = match broker_client.subscribe_responses().await {
        Ok(rx) => rx,
        Err(e) => {
            error!("Failed to subscribe to command responses: {:?}", e);
            process::exit(1);
        }
    };

    // Subscribe to DATA_BROKER telemetry signals.
    let mut telemetry_rx = match broker_client.subscribe_telemetry().await {
        Ok(rx) => rx,
        Err(e) => {
            error!("Failed to subscribe to telemetry signals: {:?}", e);
            process::exit(1);
        }
    };

    // Task 7.2: Command processing loop.
    let bearer_token = config.bearer_token.clone();
    let broker_for_cmd = broker_client.clone();
    let cmd_handle = tokio::spawn(async move {
        command_loop(&mut cmd_subscriber, &bearer_token, &broker_for_cmd).await;
    });

    // Task 7.3: Response relay loop.
    let nats_for_rsp = nats_client.clone();
    let rsp_handle = tokio::spawn(async move {
        response_relay_loop(&mut response_rx, &nats_for_rsp).await;
    });

    // Task 7.4: Telemetry loop.
    let nats_for_telem = nats_client.clone();
    let vin = config.vin.clone();
    let telem_handle = tokio::spawn(async move {
        telemetry_loop(&mut telemetry_rx, &nats_for_telem, &vin).await;
    });

    // Wait for all tasks (they run indefinitely until a connection drops).
    tokio::select! {
        result = cmd_handle => {
            if let Err(e) = result {
                error!("Command processing task failed: {:?}", e);
            }
        }
        result = rsp_handle => {
            if let Err(e) = result {
                error!("Response relay task failed: {:?}", e);
            }
        }
        result = telem_handle => {
            if let Err(e) = result {
                error!("Telemetry task failed: {:?}", e);
            }
        }
    }

    warn!("Service shutting down");
}

/// Task 7.2: Command processing loop.
///
/// Receives messages from the NATS commands subscription, validates the bearer
/// token and command payload, then writes the original payload as-is to
/// DATA_BROKER.
///
/// Implements: [04-REQ-5.1], [04-REQ-5.2], [04-REQ-5.E1], [04-REQ-5.E2],
///             [04-REQ-6.1], [04-REQ-6.2], [04-REQ-6.3], [04-REQ-6.4],
///             [04-REQ-6.E1], [04-REQ-6.E2], [04-REQ-6.E3],
///             [04-REQ-10.2], [04-REQ-10.3]
async fn command_loop(
    subscriber: &mut async_nats::Subscriber,
    bearer_token: &str,
    broker: &broker_client::BrokerClient,
) {
    use futures_util::StreamExt;

    while let Some(msg) = subscriber.next().await {
        // Extract Authorization header from NATS message headers.
        let auth_header = msg
            .headers
            .as_ref()
            .and_then(|h| h.get("Authorization"))
            .map(|v| v.as_str());

        // Validate bearer token.
        if let Err(e) = command_validator::validate_bearer_token(auth_header, bearer_token) {
            warn!("Command authentication failed: {}", e);
            continue;
        }

        // Validate command payload structure.
        let payload_bytes = msg.payload.as_ref();
        if let Err(e) = command_validator::validate_command_payload(payload_bytes) {
            warn!("Command validation failed: {}", e);
            continue;
        }

        // Write the original payload as-is to DATA_BROKER (passthrough fidelity).
        // Per [04-REQ-6.3]: the payload is written as-is, not re-serialized.
        let payload_str = match std::str::from_utf8(payload_bytes) {
            Ok(s) => s,
            Err(e) => {
                warn!("Command payload is not valid UTF-8: {}", e);
                continue;
            }
        };

        match broker.write_command(payload_str).await {
            Ok(()) => {
                info!("Validated command forwarded to DATA_BROKER");
            }
            Err(e) => {
                error!("Failed to write command to DATA_BROKER: {:?}", e);
            }
        }
    }

    warn!("Command subscription stream ended");
}

/// Task 7.3: Response relay loop.
///
/// Receives command response JSON strings from DATA_BROKER and publishes
/// them verbatim to NATS on `vehicles.{VIN}.command_responses`.
///
/// Implements: [04-REQ-7.1], [04-REQ-7.2], [04-REQ-10.2]
async fn response_relay_loop(
    response_rx: &mut tokio::sync::mpsc::Receiver<String>,
    nats_client: &nats_client::NatsClient,
) {
    while let Some(json) = response_rx.recv().await {
        info!("Relaying command response to NATS");
        if let Err(e) = nats_client.publish_response(&json).await {
            error!("Failed to relay command response to NATS: {:?}", e);
        }
    }

    warn!("Response relay channel closed");
}

/// Task 7.4: Telemetry loop.
///
/// Receives signal updates from DATA_BROKER, aggregates them via
/// `TelemetryState`, and publishes the aggregated JSON to NATS on
/// `vehicles.{VIN}.telemetry`.
///
/// Implements: [04-REQ-8.1], [04-REQ-8.2], [04-REQ-8.3], [04-REQ-10.2]
async fn telemetry_loop(
    telemetry_rx: &mut tokio::sync::mpsc::Receiver<models::SignalUpdate>,
    nats_client: &nats_client::NatsClient,
    vin: &str,
) {
    let mut state = telemetry::TelemetryState::new(vin.to_string());

    while let Some(signal_update) = telemetry_rx.recv().await {
        if let Some(json) = state.update(signal_update) {
            info!("Publishing telemetry update to NATS");
            if let Err(e) = nats_client.publish_telemetry(&json).await {
                error!("Failed to publish telemetry to NATS: {:?}", e);
            }
        }
    }

    warn!("Telemetry channel closed");
}
