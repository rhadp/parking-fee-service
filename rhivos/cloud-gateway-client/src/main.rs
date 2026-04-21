use futures::StreamExt;
use tracing::{error, info, warn};

use cloud_gateway_client::broker_client::BrokerClient;
use cloud_gateway_client::command_validator::{validate_bearer_token, validate_command_payload};
use cloud_gateway_client::config::Config;
use cloud_gateway_client::errors::ConfigError;
use cloud_gateway_client::nats_client::NatsClient;
use cloud_gateway_client::telemetry::TelemetryState;

/// Entry point.
///
/// Requires the `serve` subcommand to start the service. Without it (or with
/// unknown flags) the binary prints usage and exits 0 / 1.
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Require the "serve" subcommand per 01-REQ-4.1.
    // No args → print version (01-REQ-4.1); unknown flags → exit 1 (01-REQ-4.E1).
    if args.get(1).map(|s| s.as_str()) != Some("serve") {
        for arg in &args[1..] {
            if arg.starts_with('-') {
                eprintln!("usage: cloud-gateway-client serve");
                std::process::exit(1);
            }
        }
        println!("cloud-gateway-client v0.1.0");
        return;
    }

    // 7.5 — Initialize structured logging via tracing-subscriber.
    //
    // Log level is controlled by the RUST_LOG environment variable.
    // Validates: [04-REQ-10.1]
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // ── Startup sequence ─────────────────────────────────────────────────────
    // 7.1 / REQ-9.1: Steps execute in strict order; any failure exits with code 1
    // (REQ-9.2).

    // Step 1: Read and validate environment variables.
    let config = match Config::from_env() {
        Ok(c) => c,
        Err(ConfigError::MissingVin) => {
            eprintln!("ERROR: VIN environment variable is required but not set");
            std::process::exit(1);
        }
    };

    // Step 2: Connect to NATS with exponential-backoff retry.
    // Validates: [04-REQ-2.1], [04-REQ-2.2], [04-REQ-2.E1]
    let nats = match NatsClient::connect(&config).await {
        Ok(n) => n,
        Err(e) => {
            error!(error = %e, "Failed to connect to NATS; exiting");
            std::process::exit(1);
        }
    };

    // Step 3: Connect to DATA_BROKER.
    // Validates: [04-REQ-3.1], [04-REQ-3.E1]
    let broker = match BrokerClient::connect(&config).await {
        Ok(b) => b,
        Err(e) => {
            error!(error = %e, "Failed to connect to DATA_BROKER; exiting");
            std::process::exit(1);
        }
    };

    // Step 4: Publish self-registration.
    // Validates: [04-REQ-4.1], [04-REQ-4.2]
    if let Err(e) = nats.publish_registration().await {
        error!(error = %e, "Failed to publish self-registration; exiting");
        std::process::exit(1);
    }

    // Step 5: Subscribe to all channels before processing begins.
    let mut cmd_sub = match nats.subscribe_commands().await {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Failed to subscribe to NATS command subject; exiting");
            std::process::exit(1);
        }
    };

    // Subscribe to command responses from DATA_BROKER.
    // Validates: [04-REQ-3.3], [04-REQ-7.1]
    let mut response_rx = match broker.subscribe_responses().await {
        Ok(r) => r,
        Err(e) => {
            error!(error = %e, "Failed to subscribe to DATA_BROKER command responses; exiting");
            std::process::exit(1);
        }
    };

    // Subscribe to telemetry signals from DATA_BROKER.
    // Validates: [04-REQ-3.2], [04-REQ-8.1]
    let mut telemetry_rx = match broker.subscribe_telemetry().await {
        Ok(t) => t,
        Err(e) => {
            error!(error = %e, "Failed to subscribe to DATA_BROKER telemetry signals; exiting");
            std::process::exit(1);
        }
    };

    info!("CLOUD_GATEWAY_CLIENT ready; processing commands and telemetry");

    // ── Processing loops ─────────────────────────────────────────────────────

    let mut telemetry_state = TelemetryState::new(config.vin.clone());
    let bearer_token = config.bearer_token.clone();

    loop {
        tokio::select! {
            // 7.2 — Command processing loop.
            //
            // Each inbound NATS message is:
            //   1. Bearer-token authenticated.
            //   2. Payload-validated.
            //   3. Written verbatim to Vehicle.Command.Door.Lock in DATA_BROKER.
            //
            // Validates: [04-REQ-5.1], [04-REQ-5.2], [04-REQ-5.E1], [04-REQ-5.E2],
            //             [04-REQ-6.1], [04-REQ-6.2], [04-REQ-6.3]
            msg_opt = cmd_sub.next() => {
                let msg = match msg_opt {
                    Some(m) => m,
                    None => {
                        error!("NATS command subscription stream ended unexpectedly; exiting");
                        break;
                    }
                };

                // Extract the Authorization header value (None if absent).
                let auth_value: Option<&str> = msg.headers.as_ref()
                    .and_then(|h| h.get("Authorization"))
                    .map(|v| v.as_str());

                // Step 1: Authenticate bearer token.
                if let Err(auth_err) = validate_bearer_token(auth_value, &bearer_token) {
                    warn!(
                        error = ?auth_err,
                        "Command authentication failed; discarding message"
                    );
                    continue;
                }

                // Step 2: Validate command payload structure.
                match validate_command_payload(&msg.payload) {
                    Err(val_err) => {
                        warn!(
                            error = ?val_err,
                            "Command payload validation failed; discarding message"
                        );
                    }
                    Ok(cmd) => {
                        // Step 3: Write the payload verbatim to DATA_BROKER
                        // (Property 3: passthrough fidelity).
                        match std::str::from_utf8(&msg.payload) {
                            Ok(payload_str) => {
                                info!(
                                    command_id = %cmd.command_id,
                                    action = %cmd.action,
                                    "Forwarding validated command to DATA_BROKER"
                                );
                                if let Err(e) = broker.write_command(payload_str).await {
                                    error!(
                                        error = %e,
                                        "Failed to write command to DATA_BROKER"
                                    );
                                }
                            }
                            Err(e) => {
                                warn!(
                                    error = %e,
                                    "Command payload is not valid UTF-8; discarding"
                                );
                            }
                        }
                    }
                }
            }

            // 7.3 — Command response relay loop.
            //
            // DATA_BROKER response values (already JSON-validated by BrokerClient)
            // are published verbatim to vehicles.{VIN}.command_responses on NATS.
            //
            // Validates: [04-REQ-7.1], [04-REQ-7.2]
            response_opt = response_rx.recv() => {
                match response_opt {
                    Some(json) => {
                        info!("Relaying command response to NATS");
                        if let Err(e) = nats.publish_response(&json).await {
                            error!(error = %e, "Failed to relay command response to NATS");
                        }
                    }
                    None => {
                        error!("DATA_BROKER command response channel closed; exiting");
                        break;
                    }
                }
            }

            // 7.4 — Telemetry loop.
            //
            // Signal updates are aggregated by TelemetryState and published to
            // vehicles.{VIN}.telemetry on every change. Fields that have never
            // been set are omitted from the payload (REQ-8.3).
            //
            // Validates: [04-REQ-8.1], [04-REQ-8.2], [04-REQ-8.3]
            signal_opt = telemetry_rx.recv() => {
                match signal_opt {
                    Some(signal) => {
                        if let Some(json) = telemetry_state.update(signal) {
                            info!("Publishing telemetry update to NATS");
                            if let Err(e) = nats.publish_telemetry(&json).await {
                                error!(error = %e, "Failed to publish telemetry to NATS");
                            }
                        }
                    }
                    None => {
                        error!("DATA_BROKER telemetry channel closed; exiting");
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
