//! CLOUD_GATEWAY_CLIENT — bridges DATA_BROKER with CLOUD_GATEWAY via NATS.
//!
//! This service runs in the RHIVOS safety partition and implements three
//! data flows:
//! 1. Inbound command processing (NATS -> DATA_BROKER)
//! 2. Outbound command response relay (DATA_BROKER -> NATS)
//! 3. Outbound telemetry publishing (DATA_BROKER -> NATS)

use std::process::ExitCode;

use tokio::sync::mpsc;
use tracing::{error, info, warn};

use cloud_gateway_client::broker_client::BrokerClient;
use cloud_gateway_client::command_validator::{validate_bearer_token, validate_command_payload};
use cloud_gateway_client::config::Config;
use cloud_gateway_client::nats_client::NatsClient;
use cloud_gateway_client::telemetry::TelemetryState;

#[tokio::main]
async fn main() -> ExitCode {
    // Task 7.5: Initialize tracing-subscriber with env-filter support.
    // Defaults to INFO if RUST_LOG is not set.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    // Task 7.1: Startup sequencing — REQ-9.1, REQ-9.2
    // Step 1: Read and validate environment variables.
    let config = match Config::from_env() {
        Ok(c) => {
            info!(vin = %c.vin, "Configuration loaded");
            c
        }
        Err(e) => {
            error!(error = %e, "Failed to load configuration");
            return ExitCode::from(1);
        }
    };

    // Step 2: Connect to NATS (with exponential backoff retry).
    let nats = match NatsClient::connect(&config).await {
        Ok(n) => n,
        Err(e) => {
            error!(error = %e, "Failed to connect to NATS");
            return ExitCode::from(1);
        }
    };

    // Step 3: Connect to DATA_BROKER.
    let broker = match BrokerClient::connect(&config).await {
        Ok(b) => b,
        Err(e) => {
            error!(error = %e, "Failed to connect to DATA_BROKER");
            return ExitCode::from(1);
        }
    };

    // Step 4: Publish self-registration (fire-and-forget per REQ-4.2).
    // We log errors but do not exit since REQ-4.2 specifies fire-and-forget
    // semantics, meaning delivery failures are tolerated.
    if let Err(e) = nats.publish_registration().await {
        warn!(error = %e, "Failed to publish self-registration (fire-and-forget)");
    }

    // Step 5: Begin processing commands and telemetry.
    // Subscribe to command subject on NATS.
    let mut cmd_subscriber = match nats.subscribe_commands().await {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Failed to subscribe to commands");
            return ExitCode::from(1);
        }
    };

    // Subscribe to command responses from DATA_BROKER.
    let mut response_rx = match broker.subscribe_responses().await {
        Ok(rx) => rx,
        Err(e) => {
            error!(error = %e, "Failed to subscribe to DATA_BROKER responses");
            return ExitCode::from(1);
        }
    };

    // Subscribe to telemetry signals from DATA_BROKER.
    let mut telemetry_rx = match broker.subscribe_telemetry().await {
        Ok(rx) => rx,
        Err(e) => {
            error!(error = %e, "Failed to subscribe to DATA_BROKER telemetry");
            return ExitCode::from(1);
        }
    };

    info!("Service started, processing commands and telemetry");

    // Use channels to communicate between spawned tasks and the main select loop.
    // The broker is shared via Arc-like patterns through channels.
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<(String, String)>(32);

    // Task 7.2: Command processing loop.
    // Spawn a task that reads from NATS, validates, and forwards valid
    // command payloads for writing to DATA_BROKER.
    let bearer_token = config.bearer_token.clone();
    tokio::spawn(async move {
        command_loop(&mut cmd_subscriber, &bearer_token, cmd_tx).await;
    });

    // Task 7.3 & 7.4: Response relay and telemetry loops run in the
    // main select loop alongside command write processing.
    let mut telem_state = TelemetryState::new(config.vin.clone());

    loop {
        tokio::select! {
            // Process validated commands: write to DATA_BROKER.
            Some((original_payload, _command_id)) = cmd_rx.recv() => {
                if let Err(e) = broker.write_command(&original_payload).await {
                    error!(
                        error = %e,
                        "Failed to write command to DATA_BROKER"
                    );
                } else {
                    info!("Command forwarded to DATA_BROKER");
                }
            }

            // Task 7.3: Response relay — DATA_BROKER -> NATS.
            Some(response_json) = response_rx.recv() => {
                // Relay verbatim per REQ-7.1, Property 4.
                if let Err(e) = nats.publish_response(&response_json).await {
                    error!(
                        error = %e,
                        "Failed to relay command response to NATS"
                    );
                }
            }

            // Task 7.4: Telemetry — DATA_BROKER -> aggregate -> NATS.
            Some(signal_update) = telemetry_rx.recv() => {
                if let Some(json) = telem_state.update(signal_update) {
                    if let Err(e) = nats.publish_telemetry(&json).await {
                        error!(
                            error = %e,
                            "Failed to publish telemetry to NATS"
                        );
                    }
                }
            }

            // All channels closed — nothing left to process.
            else => {
                info!("All channels closed, shutting down");
                break;
            }
        }
    }

    ExitCode::SUCCESS
}

/// Command processing loop (Task 7.2).
///
/// Receives messages from NATS, validates bearer token and command payload,
/// and sends validated command payloads for DATA_BROKER writing.
///
/// Requirements: [04-REQ-5.1], [04-REQ-5.2], [04-REQ-5.E1], [04-REQ-5.E2],
/// [04-REQ-6.1], [04-REQ-6.2], [04-REQ-6.3], [04-REQ-6.E1], [04-REQ-6.E2],
/// [04-REQ-6.E3], [04-REQ-10.2], [04-REQ-10.3]
async fn command_loop(
    subscriber: &mut async_nats::Subscriber,
    bearer_token: &str,
    tx: mpsc::Sender<(String, String)>,
) {
    use futures::StreamExt;

    while let Some(msg) = subscriber.next().await {
        // Step 1: Extract headers and validate bearer token (REQ-5.1, REQ-5.2).
        let headers = NatsClient::extract_headers(&msg);

        if let Err(e) = validate_bearer_token(&headers, bearer_token) {
            warn!(
                error = %e,
                "Command authentication failed, discarding message"
            );
            continue;
        }

        // Step 2: Validate command payload (REQ-6.1, REQ-6.2).
        let payload_bytes = msg.payload.as_ref();
        let payload_str = match std::str::from_utf8(payload_bytes) {
            Ok(s) => s,
            Err(_) => {
                warn!("Command payload is not valid UTF-8, discarding message");
                continue;
            }
        };

        match validate_command_payload(payload_bytes) {
            Ok(cmd) => {
                info!(
                    command_id = %cmd.command_id,
                    action = %cmd.action,
                    "Validated command, forwarding to DATA_BROKER"
                );
                // Step 3: Send original payload as-is for writing (REQ-6.3,
                // Property 3: Command Passthrough Fidelity).
                if tx
                    .send((payload_str.to_string(), cmd.command_id.clone()))
                    .await
                    .is_err()
                {
                    info!("Command channel closed, stopping command loop");
                    return;
                }
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Command validation failed, discarding message"
                );
            }
        }
    }

    info!("NATS command subscription ended");
}
