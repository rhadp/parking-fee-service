//! RHIVOS Cloud Gateway Client.
//!
//! This service bridges the vehicle to the cloud via MQTT (Eclipse Mosquitto).
//! It connects to the MQTT broker, registers the vehicle by publishing its VIN
//! and pairing PIN, subscribes to command and status request topics, processes
//! commands via DATA_BROKER, forwards lock results back to MQTT, handles
//! status requests, and periodically publishes telemetry.
//!
//! # Startup sequence
//!
//! 1. Parse configuration from CLI args / environment variables.
//! 2. Load or generate VIN and pairing PIN (persisted to data directory).
//! 3. Connect to MQTT broker and subscribe to vehicle topics.
//! 4. Publish registration message to CLOUD_GATEWAY.
//! 5. Spawn command handler (MQTT event loop dispatch), result forwarder,
//!    and telemetry publisher.
//! 6. Run until shutdown.
//!
//! # Shutdown
//!
//! The service shuts down gracefully on SIGINT or SIGTERM.
//!
//! # Requirements
//!
//! - 03-REQ-3.1: Subscribe to vehicle command and status topics.
//! - 03-REQ-3.2: Lock command → write `Vehicle.Command.Door.Lock = true`.
//! - 03-REQ-3.3: Unlock command → write `Vehicle.Command.Door.Lock = false`.
//! - 03-REQ-3.4: Subscribe to LockResult, publish CommandResponse.
//! - 03-REQ-3.5: Handle status requests, read DATA_BROKER, publish response.
//! - 03-REQ-3.6: Accept MQTT and DATA_BROKER addresses via CLI / env vars.
//! - 03-REQ-3.E1: DATA_BROKER unreachable → retry with exponential backoff.
//! - 03-REQ-3.E2: MQTT connection lost → reconnect and re-subscribe.
//! - 03-REQ-3.E3: Invalid command JSON → log and discard.
//! - 03-REQ-4.1: Periodically publish telemetry to MQTT (QoS 0).
//! - 03-REQ-4.2: Telemetry includes all required vehicle signals.
//! - 03-REQ-4.3: Telemetry interval configurable via CLI / env var.
//! - 03-REQ-5.1: Generate VIN and PIN on first start, persist, log to stdout.
//! - 03-REQ-5.2: Publish registration message on every startup.
//! - 03-REQ-5.E3: Reuse persisted VIN and PIN on subsequent starts.

pub mod command_handler;
pub mod config;
pub mod messages;
pub mod mqtt;
pub mod result_forwarder;
pub mod status_handler;
pub mod telemetry;
pub mod vin;

use clap::Parser;
use parking_proto::kuksa_client::KuksaClient;
use parking_proto::signals;
use std::path::Path;
use tokio::signal;
use tracing::{error, info, warn};

use crate::command_handler::DataBrokerWriter;
use crate::result_forwarder::LockResultSubscriber;
use crate::status_handler::DataBrokerReader;

// ---------------------------------------------------------------------------
// Kuksa adapter — implements the traits needed by command_handler,
// result_forwarder, and status_handler using parking_proto::KuksaClient.
// ---------------------------------------------------------------------------

/// Adapter bridging the abstract `DataBrokerWriter`, `DataBrokerReader`, and
/// `LockResultSubscriber` traits to the concrete `KuksaClient`.
#[derive(Clone)]
pub struct KuksaAdapter {
    client: KuksaClient,
}

impl KuksaAdapter {
    pub fn new(client: KuksaClient) -> Self {
        Self { client }
    }
}

impl DataBrokerWriter for KuksaAdapter {
    async fn set_door_lock(&self, lock: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .set_bool(signals::COMMAND_DOOR_LOCK, lock)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

impl DataBrokerReader for KuksaAdapter {
    async fn get_is_locked(&self) -> Result<Option<bool>, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .get_bool(signals::DOOR_IS_LOCKED)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_is_door_open(&self) -> Result<Option<bool>, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .get_bool(signals::DOOR_IS_OPEN)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_speed(&self) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .get_f64(signals::SPEED)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_latitude(&self) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .get_f64(signals::LOCATION_LAT)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_longitude(&self) -> Result<Option<f64>, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .get_f64(signals::LOCATION_LON)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_parking_session_active(
        &self,
    ) -> Result<Option<bool>, Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .get_bool(signals::PARKING_SESSION_ACTIVE)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

impl LockResultSubscriber for KuksaAdapter {
    async fn subscribe_lock_result(
        &self,
    ) -> Result<
        Box<
            dyn tokio_stream::Stream<Item = Result<String, Box<dyn std::error::Error + Send + Sync>>>
                + Send
                + Unpin,
        >,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        use tokio_stream::StreamExt;

        let stream = self
            .client
            .subscribe_string(signals::LOCK_RESULT)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // Map parking_proto::kuksa_client::Error to Box<dyn Error + Send>.
        let mapped = stream.map(|item| {
            item.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        });

        Ok(Box::new(Box::pin(mapped)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = config::Config::parse();

    info!(
        mqtt_addr = %config.mqtt_addr,
        databroker_addr = %config.databroker_addr,
        data_dir = %config.data_dir,
        telemetry_interval_secs = config.telemetry_interval,
        "cloud-gateway-client starting"
    );

    // 1. Load or generate VIN and pairing PIN.
    let vin_data = vin::load_or_create(Path::new(&config.data_dir)).map_err(|e| {
        error!(error = %e, "failed to load/generate VIN");
        e
    })?;

    info!(
        vin = %vin_data.vin,
        pairing_pin = %vin_data.pairing_pin,
        "vehicle identity ready"
    );

    // 2. Connect to MQTT and publish registration.
    let (mqtt_client, event_loop) =
        mqtt::connect_and_register(&config.mqtt_addr, &vin_data)
            .await
            .map_err(|e| {
                error!(error = %e, "failed to connect to MQTT broker");
                e
            })?;

    info!("MQTT client connected and registration published");

    // 3. Set up command processing state.
    let pending = command_handler::new_pending_state();
    let vin = vin_data.vin.clone();

    // 4. Run the MQTT event loop with command dispatch, result forwarder,
    //    telemetry publisher, and shutdown handler as concurrent tasks.
    //
    //    The Kuksa connection is attempted here. If it fails, we still run
    //    the MQTT event loop (commands will fail gracefully). The result
    //    forwarder and telemetry publisher are only spawned if Kuksa connects
    //    successfully.
    tokio::select! {
        _ = run_mqtt_event_loop_with_dispatch(
            event_loop,
            mqtt_client.clone(),
            pending.clone(),
            &vin,
            &config.databroker_addr,
            config.telemetry_interval,
        ) => {
            error!("MQTT event loop exited unexpectedly");
        }
        _ = shutdown_signal() => {
            info!("cloud-gateway-client shutting down");
        }
    }

    Ok(())
}

/// Run the MQTT event loop, dispatching incoming messages to the command
/// handler and status handler.
///
/// Also attempts to connect to Kuksa DATA_BROKER and spawn the result
/// forwarder and telemetry publisher as background tasks.
async fn run_mqtt_event_loop_with_dispatch(
    mut event_loop: rumqttc::EventLoop,
    mqtt_client: mqtt::MqttClient,
    pending: command_handler::PendingCommandState,
    vin: &str,
    databroker_addr: &str,
    telemetry_interval_secs: u64,
) {
    use rumqttc::{Event, Packet};

    // Try to connect to Kuksa DATA_BROKER.
    let kuksa = match connect_kuksa_with_backoff(databroker_addr).await {
        Some(client) => {
            info!("connected to Kuksa DATA_BROKER");
            let adapter = KuksaAdapter::new(client);
            Some(adapter)
        }
        None => {
            warn!("could not connect to DATA_BROKER, commands will not be processed");
            None
        }
    };

    // Spawn result forwarder if Kuksa is available.
    if let Some(ref adapter) = kuksa {
        let adapter_clone = adapter.clone();
        let mqtt_clone = mqtt_client.clone();
        let pending_clone = pending.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) =
                    result_forwarder::run_result_forwarder(&adapter_clone, &mqtt_clone, &pending_clone)
                        .await
                {
                    error!(error = %e, "result forwarder error, restarting...");
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
    }

    // Spawn telemetry publisher if Kuksa is available.
    if let Some(ref adapter) = kuksa {
        let adapter_clone = adapter.clone();
        let mqtt_clone = mqtt_client.clone();
        let vin_owned = vin.to_string();
        tokio::spawn(async move {
            telemetry::run_telemetry_publisher(
                &adapter_clone,
                &mqtt_clone,
                &vin_owned,
                telemetry_interval_secs,
            )
            .await;
        });
    } else {
        warn!("telemetry publisher not started: DATA_BROKER not connected");
    }

    // Commands and status request topics for matching.
    let commands_topic = messages::topic_for(messages::TOPIC_COMMANDS, vin);
    let status_req_topic = messages::topic_for(messages::TOPIC_STATUS_REQUEST, vin);

    loop {
        match event_loop.poll().await {
            Ok(Event::Incoming(incoming)) => {
                match incoming {
                    Packet::ConnAck(_) => {
                        info!("MQTT connected");
                    }
                    Packet::SubAck(_) => {
                        // Subscription acknowledgements are expected.
                    }
                    Packet::Publish(p) => {
                        if p.topic == commands_topic {
                            if let Some(ref adapter) = kuksa {
                                command_handler::handle_command(
                                    &p.payload,
                                    &pending,
                                    adapter,
                                )
                                .await;
                            } else {
                                warn!(
                                    topic = %p.topic,
                                    "received command but DATA_BROKER not connected"
                                );
                            }
                        } else if p.topic == status_req_topic {
                            if let Some(ref adapter) = kuksa {
                                status_handler::handle_status_request(
                                    &p.payload,
                                    vin,
                                    adapter,
                                    &mqtt_client,
                                )
                                .await;
                            } else {
                                warn!(
                                    topic = %p.topic,
                                    "received status request but DATA_BROKER not connected"
                                );
                            }
                        } else {
                            info!(topic = %p.topic, "received unhandled MQTT message");
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Outgoing(_)) => {
                // Outgoing events (PubAck, PubRec, etc.) — no action needed.
            }
            Err(e) => {
                warn!(error = %e, "MQTT connection error, will retry...");
                // rumqttc auto-reconnects; sleep briefly to avoid tight loop.
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}

/// Attempt to connect to Kuksa DATA_BROKER with exponential backoff.
///
/// Makes up to 5 attempts before giving up (to avoid blocking startup
/// indefinitely if Kuksa is not available).
async fn connect_kuksa_with_backoff(addr: &str) -> Option<KuksaClient> {
    const MAX_ATTEMPTS: u32 = 5;
    let mut backoff_secs = 1u64;

    for attempt in 1..=MAX_ATTEMPTS {
        match KuksaClient::connect(addr).await {
            Ok(client) => return Some(client),
            Err(e) => {
                warn!(
                    attempt,
                    max_attempts = MAX_ATTEMPTS,
                    error = %e,
                    "failed to connect to Kuksa DATA_BROKER, retrying in {backoff_secs}s..."
                );
                tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(30);
            }
        }
    }

    error!("exhausted {MAX_ATTEMPTS} attempts to connect to Kuksa DATA_BROKER");
    None
}

/// Wait for a shutdown signal (SIGINT or SIGTERM).
async fn shutdown_signal() {
    let ctrl_c = signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm =
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to register SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {},
            _ = sigterm.recv() => {},
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use clap::Parser;

    #[test]
    fn cli_parses_default_args() {
        let config = Config::parse_from(["cloud-gateway-client"]);
        assert_eq!(config.mqtt_addr, "localhost:1883");
        assert_eq!(config.databroker_addr, "http://localhost:55555");
        assert_eq!(config.data_dir, "./data");
        assert_eq!(config.telemetry_interval, 5);
    }

    #[test]
    fn cli_parses_custom_mqtt_addr() {
        let config = Config::parse_from([
            "cloud-gateway-client",
            "--mqtt-addr",
            "mosquitto:1883",
        ]);
        assert_eq!(config.mqtt_addr, "mosquitto:1883");
    }

    #[test]
    fn cli_parses_all_custom_args() {
        let config = Config::parse_from([
            "cloud-gateway-client",
            "--mqtt-addr",
            "mqtt:1883",
            "--databroker-addr",
            "http://kuksa:55555",
            "--data-dir",
            "/tmp/data",
            "--telemetry-interval",
            "10",
        ]);
        assert_eq!(config.mqtt_addr, "mqtt:1883");
        assert_eq!(config.databroker_addr, "http://kuksa:55555");
        assert_eq!(config.data_dir, "/tmp/data");
        assert_eq!(config.telemetry_interval, 10);
    }

    /// Verify KuksaAdapter implements all required traits.
    #[test]
    fn kuksa_adapter_is_clone() {
        fn _assert_clone<T: Clone>() {}
        _assert_clone::<crate::KuksaAdapter>();
    }
}
