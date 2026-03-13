use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::{error, info, warn};

mod broker;
mod command;
mod config;
mod nats_client;
mod relay;
mod telemetry;

#[cfg(test)]
mod proptest_cases;
#[cfg(test)]
mod testing;

use broker::{BrokerUpdate, BrokerValue, GrpcBrokerClient};
use command::{parse_and_validate_command, validate_bearer_token};
use config::load_config;
use nats_client::{NatsClient, NatsPublisher as _};
use relay::{forward_command, publish_telemetry, relay_response};
use telemetry::TelemetryState;

/// VSS signal path for door response.
const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// VSS signal paths for telemetry subscription.
const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
const SIGNAL_LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";
const SIGNAL_LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";
const SIGNAL_PARKING: &str = "Vehicle.Parking.SessionActive";

const TELEMETRY_SIGNALS: &[&str] =
    &[SIGNAL_IS_LOCKED, SIGNAL_LATITUDE, SIGNAL_LONGITUDE, SIGNAL_PARKING];

/// Service version from Cargo.toml.
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    // Initialise structured logging.
    tracing_subscriber::fmt::init();

    // Load configuration — exit with code 1 if VIN is missing (04-REQ-6.E1).
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Fatal: configuration error: {e}");
            std::process::exit(1);
        }
    };

    // Log startup metadata (04-REQ-7.2).
    info!(
        version = VERSION,
        vin = %config.vin,
        nats_url = %config.nats_url,
        databroker_addr = %config.databroker_addr,
        "cloud-gateway-client starting"
    );

    // Connect to NATS with retry (04-REQ-1.1, 04-REQ-1.E1).
    let nats = match NatsClient::connect(&config.nats_url).await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!(error = %e, "Failed to connect to NATS — exiting");
            std::process::exit(1);
        }
    };

    // Connect to DATA_BROKER with retry (04-REQ-5.1, 04-REQ-5.E1).
    let broker = match GrpcBrokerClient::connect(&config.databroker_addr).await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!(error = %e, "Failed to connect to DATA_BROKER — exiting");
            std::process::exit(1);
        }
    };

    // Publish self-registration message (04-REQ-6.2).
    let ts = unix_now();
    let reg_payload = format!(
        r#"{{"vin":"{}","status":"online","timestamp":{}}}"#,
        config.vin, ts
    );
    let status_subject = format!("vehicles.{}.status", config.vin);
    if let Err(e) = nats.publish(&status_subject, reg_payload.as_bytes()).await {
        error!(error = %e, "Failed to publish registration message");
    }

    // Subscribe to NATS commands (04-REQ-1.2).
    let cmd_subject = format!("vehicles.{}.commands", config.vin);
    let cmd_sub = match nats.subscribe(&cmd_subject).await {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, subject = %cmd_subject, "Failed to subscribe to NATS commands — exiting");
            std::process::exit(1);
        }
    };

    // Subscribe to DATA_BROKER signals (04-REQ-3.1, 04-REQ-4.1).
    let response_rx = broker.subscribe_signals(&[SIGNAL_RESPONSE]).await;
    let telemetry_rx = broker.subscribe_signals(TELEMETRY_SIGNALS).await;

    info!(vin = %config.vin, "cloud-gateway-client ready");

    // Shutdown watch channel: main sends `true` on SIGTERM/SIGINT.
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // ── Task 1: Command handler ──────────────────────────────────────────────
    // Reads NATS commands, validates token + payload, forwards to DATA_BROKER.
    let broker_for_cmd = Arc::clone(&broker);
    let token_for_cmd = config.bearer_token.clone();
    let mut shutdown_for_cmd = shutdown_rx.clone();
    let mut cmd_sub = cmd_sub;
    let cmd_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = shutdown_for_cmd.changed() => {
                    info!("Command handler: shutdown signal received");
                    break;
                }
                msg = cmd_sub.next() => {
                    match msg {
                        None => {
                            warn!("Command handler: NATS subscription closed");
                            break;
                        }
                        Some(msg) => {
                            // Validate bearer token (04-REQ-2.1, 04-REQ-2.E1).
                            let auth = msg
                                .headers
                                .as_ref()
                                .and_then(|h| h.get("Authorization"))
                                .map(|v| v.as_str());
                            if !validate_bearer_token(auth, &token_for_cmd) {
                                warn!("Command rejected: invalid or missing bearer token");
                                continue;
                            }

                            // Parse and validate command JSON (04-REQ-2.2, 04-REQ-2.E2, 04-REQ-2.E3).
                            if let Err(e) = parse_and_validate_command(&msg.payload) {
                                warn!(error = ?e, "Command rejected: validation failed");
                                continue;
                            }

                            // Forward the original JSON verbatim to DATA_BROKER (04-REQ-2.3).
                            let payload_str = match std::str::from_utf8(&msg.payload) {
                                Ok(s) => s,
                                Err(e) => {
                                    warn!(error = %e, "Command payload is not valid UTF-8");
                                    continue;
                                }
                            };
                            if let Err(e) =
                                forward_command(&*broker_for_cmd, payload_str).await
                            {
                                error!(error = %e, "Failed to forward command to DATA_BROKER");
                            }
                        }
                    }
                }
            }
        }
        info!("Command handler: exited");
    });

    // ── Task 2: Response relay ───────────────────────────────────────────────
    // Reads Vehicle.Command.Door.Response from DATA_BROKER, publishes to NATS.
    let nats_for_rsp = Arc::clone(&nats);
    let vin_for_rsp = config.vin.clone();
    let mut shutdown_for_rsp = shutdown_rx.clone();
    let mut response_rx = response_rx;
    let rsp_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = shutdown_for_rsp.changed() => {
                    info!("Response relay: shutdown signal received");
                    break;
                }
                update = response_rx.recv() => {
                    match update {
                        None => {
                            // Placeholder: subscription closed immediately.
                            // Real gRPC implementation wired in task group 6.
                            shutdown_for_rsp.changed().await.ok();
                            break;
                        }
                        Some(BrokerUpdate { value: BrokerValue::String(json), .. }) => {
                            relay_response(nats_for_rsp.as_ref(), &vin_for_rsp, &json).await;
                        }
                        Some(update) => {
                            warn!(
                                path = %update.path,
                                "Response relay: unexpected value type for response signal"
                            );
                        }
                    }
                }
            }
        }
        info!("Response relay: exited");
    });

    // ── Task 3: Telemetry publisher ─────────────────────────────────────────
    // Aggregates DATA_BROKER signal changes and publishes telemetry to NATS.
    let nats_for_tel = Arc::clone(&nats);
    let vin_for_tel = config.vin.clone();
    let mut shutdown_for_tel = shutdown_rx.clone();
    let mut telemetry_rx = telemetry_rx;
    let tel_task = tokio::spawn(async move {
        let mut state = TelemetryState::default();
        loop {
            tokio::select! {
                biased;
                _ = shutdown_for_tel.changed() => {
                    info!("Telemetry publisher: shutdown signal received");
                    break;
                }
                update = telemetry_rx.recv() => {
                    match update {
                        None => {
                            // Placeholder: subscription closed immediately.
                            // Real gRPC implementation wired in task group 6.
                            shutdown_for_tel.changed().await.ok();
                            break;
                        }
                        Some(BrokerUpdate { path, value }) => {
                            // Update aggregated state (04-REQ-4.2, 04-REQ-4.3).
                            match path.as_str() {
                                SIGNAL_IS_LOCKED => {
                                    if let BrokerValue::Bool(b) = value {
                                        state.is_locked = Some(b);
                                    }
                                }
                                SIGNAL_LATITUDE => {
                                    if let BrokerValue::Float(f) = value {
                                        state.latitude = Some(f);
                                    }
                                }
                                SIGNAL_LONGITUDE => {
                                    if let BrokerValue::Float(f) = value {
                                        state.longitude = Some(f);
                                    }
                                }
                                SIGNAL_PARKING => {
                                    if let BrokerValue::Bool(b) = value {
                                        state.parking_active = Some(b);
                                    }
                                }
                                _ => {
                                    warn!(path, "Telemetry: unexpected signal path");
                                }
                            }
                            publish_telemetry(nats_for_tel.as_ref(), &vin_for_tel, &state)
                                .await;
                        }
                    }
                }
            }
        }
        info!("Telemetry publisher: exited");
    });

    // ── Wait for shutdown signal ─────────────────────────────────────────────
    // Handle SIGTERM and SIGINT for graceful lifecycle (04-REQ-7.1).
    wait_for_shutdown_signal().await;
    info!("Shutdown signal received — initiating graceful shutdown");

    // Broadcast shutdown to all tasks (04-REQ-7.E1: in-flight command completes
    // before tasks exit because shutdown is checked after message processing).
    let _ = shutdown_tx.send(true);

    // Wait up to 5 s for tasks to drain in-flight work (04-REQ-7.E1).
    let drain = async {
        let _ = cmd_task.await;
        let _ = rsp_task.await;
        let _ = tel_task.await;
    };
    if tokio::time::timeout(std::time::Duration::from_secs(5), drain)
        .await
        .is_err()
    {
        warn!("Shutdown timed out waiting for tasks — forcing exit");
    }

    // Flush any pending NATS publishes (04-REQ-7.1).
    if let Err(e) = nats.flush().await {
        warn!(error = %e, "NATS flush failed during shutdown");
    }

    info!("cloud-gateway-client shutdown complete");
    std::process::exit(0);
}

/// Block until SIGTERM or SIGINT is received.
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

/// Return the current Unix timestamp in seconds.
fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
