//! Main CLOUD_GATEWAY_CLIENT service logic.
//!
//! Orchestrates the MQTT client, command relay, response relay, and
//! telemetry publishing tasks. Connects to both the MQTT broker and
//! DATA_BROKER, then runs all tasks concurrently.

use databroker_client::{DataValue, DatabrokerClient};
use tracing::{error, info, warn};

use crate::commands::validate_command;
use crate::mqtt::MqttClient;
use crate::telemetry;

/// VSS signal path for the command signal.
const COMMAND_SIGNAL: &str = "Vehicle.Command.Door.Lock";

/// Configuration for the CLOUD_GATEWAY_CLIENT service.
pub struct ServiceConfig {
    /// DATA_BROKER endpoint (UDS path or TCP address).
    pub databroker_endpoint: String,
    /// MQTT broker host.
    pub mqtt_host: String,
    /// MQTT broker port.
    pub mqtt_port: u16,
    /// Vehicle identification number.
    pub vin: String,
}

/// Run the CLOUD_GATEWAY_CLIENT service.
///
/// This function connects to both the MQTT broker and DATA_BROKER,
/// then runs the command relay, response relay, and telemetry publishing
/// tasks concurrently.
pub async fn run(config: ServiceConfig) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        databroker = %config.databroker_endpoint,
        mqtt_host = %config.mqtt_host,
        mqtt_port = config.mqtt_port,
        vin = %config.vin,
        "starting cloud-gateway-client"
    );

    // Connect to DATA_BROKER
    info!(endpoint = %config.databroker_endpoint, "connecting to DATA_BROKER");
    let db_client = connect_databroker_with_retry(&config.databroker_endpoint).await;

    // Create MQTT client
    let mut mqtt = MqttClient::new(&config.mqtt_host, config.mqtt_port, &config.vin);
    let mqtt_publisher = mqtt.client();

    // Get topic names
    let telemetry_topic = mqtt.telemetry_topic();
    let response_topic = mqtt.response_topic();
    let command_topic = mqtt.command_topic();

    // Clone DB client for the command handler
    let db_for_commands = db_client.clone();

    // Spawn telemetry subscription task
    let db_for_telemetry = db_client.clone();
    let mqtt_for_telemetry = mqtt_publisher.clone();
    let telemetry_topic_clone = telemetry_topic.clone();
    tokio::spawn(async move {
        telemetry::run_telemetry_loop(&db_for_telemetry, &mqtt_for_telemetry, &telemetry_topic_clone)
            .await;
    });

    // Spawn response relay task
    let db_for_responses = db_client.clone();
    let mqtt_for_responses = mqtt_publisher.clone();
    let response_topic_clone = response_topic.clone();
    tokio::spawn(async move {
        telemetry::run_response_relay(&db_for_responses, &mqtt_for_responses, &response_topic_clone)
            .await;
    });

    info!("all tasks started, running MQTT event loop");

    // Run MQTT event loop with command handler
    mqtt.run(move |topic, payload| {
        if topic == command_topic {
            if let Some(validated_json) = validate_command(&payload) {
                let db = db_for_commands.clone();
                tokio::spawn(async move {
                    if let Err(e) = db
                        .set_value(
                            COMMAND_SIGNAL,
                            DataValue::String(validated_json),
                        )
                        .await
                    {
                        error!(error = %e, "failed to write command to DATA_BROKER");
                    }
                });
            }
        }
    })
    .await;

    warn!("cloud-gateway-client event loop ended");
    Ok(())
}

/// Connect to DATA_BROKER with retry and exponential backoff.
async fn connect_databroker_with_retry(endpoint: &str) -> DatabrokerClient {
    let mut retry_count: u32 = 0;

    loop {
        match DatabrokerClient::connect(endpoint).await {
            Ok(client) => {
                info!(endpoint = %endpoint, "connected to DATA_BROKER");
                return client;
            }
            Err(e) => {
                retry_count = retry_count.saturating_add(1);
                let backoff = std::cmp::min(1u64 << retry_count.min(5), 32);
                warn!(
                    error = %e,
                    endpoint = %endpoint,
                    retry = retry_count,
                    backoff_secs = backoff,
                    "DATA_BROKER connection failed, retrying"
                );
                tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
            }
        }
    }
}
