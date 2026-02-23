//! CLOUD_GATEWAY_CLIENT integration tests
//!
//! These tests verify CLOUD_GATEWAY_CLIENT MQTT connectivity, command relay,
//! response relay, and telemetry publishing.
//!
//! Test Spec: TS-02-15, TS-02-16, TS-02-17, TS-02-18, TS-02-19, TS-02-20

use std::time::Duration;

use databroker_client::{DataValue, DatabrokerClient};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};

/// Helper: check if MQTT broker is available.
fn mqtt_available() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:1883".parse().unwrap(),
        Duration::from_secs(2),
    )
    .is_ok()
}

/// Helper: check if DATA_BROKER is available.
fn databroker_available() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:55556".parse().unwrap(),
        Duration::from_secs(2),
    )
    .is_ok()
}

macro_rules! require_infra {
    () => {
        if !mqtt_available() {
            eprintln!("SKIP: MQTT broker not available on port 1883 (run `make infra-up`)");
            return;
        }
        if !databroker_available() {
            eprintln!("SKIP: DATA_BROKER not available on port 55556 (run `make infra-up`)");
            return;
        }
    };
}

/// Connect to DATA_BROKER via TCP for testing.
async fn test_db_client() -> DatabrokerClient {
    DatabrokerClient::connect("http://localhost:55556")
        .await
        .expect("should connect to DATA_BROKER on port 55556")
}

/// Create a test MQTT client with a unique client ID.
fn create_test_mqtt(client_id: &str) -> (AsyncClient, rumqttc::EventLoop) {
    let mut opts = MqttOptions::new(client_id, "localhost", 1883);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_clean_session(true);
    AsyncClient::new(opts, 64)
}

/// Wait for MQTT connection.
async fn mqtt_connect(eventloop: &mut rumqttc::EventLoop) {
    let _ = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => break,
                Ok(_) => continue,
                Err(e) => panic!("MQTT connection failed: {}", e),
            }
        }
    })
    .await;
}

/// Wait for a message on an MQTT event loop with timeout.
async fn mqtt_wait_for_message(
    eventloop: &mut rumqttc::EventLoop,
    timeout: Duration,
) -> Option<(String, Vec<u8>)> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return None;
        }
        match tokio::time::timeout(remaining, eventloop.poll()).await {
            Ok(Ok(Event::Incoming(Packet::Publish(publish)))) => {
                return Some((publish.topic.clone(), publish.payload.to_vec()));
            }
            Ok(Ok(_)) => continue,
            Ok(Err(_)) => continue,
            Err(_) => return None,
        }
    }
}

/// Spawn the cloud-gateway-client service as a background task.
fn spawn_cgc_service(
    db_client: DatabrokerClient,
    vin: &str,
) -> tokio::task::JoinHandle<()> {
    let vin = vin.to_string();
    tokio::spawn(async move {
        let mut mqtt = cloud_gateway_client::mqtt::MqttClient::new("localhost", 1883, &vin);
        let mqtt_publisher = mqtt.client();
        let telemetry_topic = mqtt.telemetry_topic();
        let response_topic = mqtt.response_topic();
        let command_topic = mqtt.command_topic();
        let db_for_commands = db_client.clone();

        let db_for_telemetry = db_client.clone();
        let mqtt_for_telemetry = mqtt_publisher.clone();
        let tt = telemetry_topic.clone();
        tokio::spawn(async move {
            cloud_gateway_client::telemetry::run_telemetry_loop(
                &db_for_telemetry,
                &mqtt_for_telemetry,
                &tt,
            )
            .await;
        });

        let db_for_responses = db_client.clone();
        let mqtt_for_responses = mqtt_publisher.clone();
        let rt = response_topic.clone();
        tokio::spawn(async move {
            cloud_gateway_client::telemetry::run_response_relay(
                &db_for_responses,
                &mqtt_for_responses,
                &rt,
            )
            .await;
        });

        mqtt.run(move |topic, payload| {
            if topic == command_topic {
                if let Some(validated_json) =
                    cloud_gateway_client::commands::validate_command(&payload)
                {
                    let db = db_for_commands.clone();
                    tokio::spawn(async move {
                        let _ = db
                            .set_value(
                                "Vehicle.Command.Door.Lock",
                                DataValue::String(validated_json),
                            )
                            .await;
                    });
                }
            }
        })
        .await;
    })
}

/// Build a command JSON string for testing.
fn make_command(command_id: &str, action: &str) -> String {
    serde_json::json!({
        "command_id": command_id,
        "action": action,
        "doors": ["driver"],
        "source": "test",
        "vin": "VIN12345",
        "timestamp": 1700000000
    })
    .to_string()
}

/// Set vehicle speed via DATA_BROKER.
async fn set_speed(client: &DatabrokerClient, speed: f32) {
    client
        .set_value("Vehicle.Speed", DataValue::Float(speed))
        .await
        .expect("should set Vehicle.Speed");
}

// ── TS-02-15: CGC connects to MQTT broker ────────────────────────────────────

/// TS-02-15: CLOUD_GATEWAY_CLIENT connects to the configured MQTT broker (02-REQ-4.1)
#[tokio::test]
async fn test_cgc_connects_to_mqtt() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client, "VIN12345");

    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(!handle.is_finished(), "CGC service should still be running");

    handle.abort();
}

// ── TS-02-16: CGC subscribes to command topic ────────────────────────────────

/// TS-02-16: CLOUD_GATEWAY_CLIENT subscribes to `vehicles/{vin}/commands` (02-REQ-4.2)
#[tokio::test]
async fn test_cgc_subscribes_to_command_topic() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let (pub_client, mut pub_loop) = create_test_mqtt("test-pub-s16");
    mqtt_connect(&mut pub_loop).await;

    let cmd = make_command("mqtt-test-s16", "lock");
    pub_client
        .publish("vehicles/VIN12345/commands", QoS::AtLeastOnce, false, cmd.as_bytes())
        .await
        .unwrap();

    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match pub_loop.poll().await {
                Ok(Event::Incoming(Packet::PubAck(_))) => break,
                Ok(_) => continue,
                Err(e) => panic!("MQTT error: {}", e),
            }
        }
    })
    .await;

    tokio::time::sleep(Duration::from_secs(2)).await;

    let value = db_client
        .get_value_opt("Vehicle.Command.Door.Lock")
        .await
        .ok()
        .flatten();

    handle.abort();

    if let Some(DataValue::String(v)) = value {
        let parsed: serde_json::Value = serde_json::from_str(&v).unwrap();
        assert_eq!(parsed["command_id"], "mqtt-test-s16");
    } else {
        panic!("expected command to appear in DATA_BROKER");
    }
}

// ── TS-02-17: CGC writes validated command to DATA_BROKER ────────────────────

/// TS-02-17: CLOUD_GATEWAY_CLIENT writes validated MQTT commands to DATA_BROKER (02-REQ-4.3)
#[tokio::test]
async fn test_cgc_writes_command_to_databroker() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let (pub_client, mut pub_loop) = create_test_mqtt("test-pub-s17");
    mqtt_connect(&mut pub_loop).await;

    let cmd = make_command("relay-test-s17", "unlock");
    pub_client
        .publish("vehicles/VIN12345/commands", QoS::AtLeastOnce, false, cmd.as_bytes())
        .await
        .unwrap();

    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match pub_loop.poll().await {
                Ok(Event::Incoming(Packet::PubAck(_))) => break,
                Ok(_) => continue,
                Err(e) => panic!("MQTT error: {}", e),
            }
        }
    })
    .await;

    tokio::time::sleep(Duration::from_secs(2)).await;

    let value = db_client
        .get_value_opt("Vehicle.Command.Door.Lock")
        .await
        .ok()
        .flatten();

    handle.abort();

    if let Some(DataValue::String(v)) = value {
        let parsed: serde_json::Value = serde_json::from_str(&v).unwrap();
        assert_eq!(parsed["command_id"], "relay-test-s17");
        assert_eq!(parsed["action"], "unlock");
    } else {
        panic!("expected command to appear in DATA_BROKER");
    }
}

// ── TS-02-18: CGC relays response to MQTT ────────────────────────────────────

/// TS-02-18: CLOUD_GATEWAY_CLIENT relays command responses to MQTT (02-REQ-4.4)
#[tokio::test]
async fn test_cgc_relays_response_to_mqtt() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let (sub_client, mut sub_loop) = create_test_mqtt("test-sub-s18");
    mqtt_connect(&mut sub_loop).await;

    sub_client
        .subscribe("vehicles/VIN12345/command_responses", QoS::AtLeastOnce)
        .await
        .unwrap();

    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match sub_loop.poll().await {
                Ok(Event::Incoming(Packet::SubAck(_))) => break,
                Ok(_) => continue,
                Err(e) => panic!("MQTT error: {}", e),
            }
        }
    })
    .await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let response_json = serde_json::json!({
        "command_id": "e2e-test-s18",
        "status": "success",
        "timestamp": 1700000000
    })
    .to_string();

    db_client
        .set_value(
            "Vehicle.Command.Door.Response",
            DataValue::String(response_json),
        )
        .await
        .expect("should write response to DATA_BROKER");

    let msg = mqtt_wait_for_message(&mut sub_loop, Duration::from_secs(10)).await;

    handle.abort();

    let (topic, payload) = msg.expect("should receive response on MQTT");
    assert_eq!(topic, "vehicles/VIN12345/command_responses");
    let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(parsed["command_id"], "e2e-test-s18");
    assert_eq!(parsed["status"], "success");
}

// ── TS-02-19: CGC subscribes to vehicle state signals ────────────────────────

/// TS-02-19: CLOUD_GATEWAY_CLIENT subscribes to state signals (02-REQ-5.1)
#[tokio::test]
async fn test_cgc_subscribes_to_state_signals() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let (sub_client, mut sub_loop) = create_test_mqtt("test-sub-s19");
    mqtt_connect(&mut sub_loop).await;

    sub_client
        .subscribe("vehicles/VIN12345/telemetry", QoS::AtLeastOnce)
        .await
        .unwrap();

    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match sub_loop.poll().await {
                Ok(Event::Incoming(Packet::SubAck(_))) => break,
                Ok(_) => continue,
                Err(e) => panic!("MQTT error: {}", e),
            }
        }
    })
    .await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    set_speed(&db_client, 42.0).await;
    let msg = mqtt_wait_for_message(&mut sub_loop, Duration::from_secs(10)).await;

    handle.abort();

    let (_topic, payload) = msg.expect("should receive telemetry on MQTT");
    let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert!(parsed["signals"].is_array());
    let signals = parsed["signals"].as_array().unwrap();
    assert!(
        signals.iter().any(|s| s["path"].as_str().unwrap_or("").contains("Speed")),
        "telemetry should contain Speed signal"
    );
}

// ── TS-02-20: CGC publishes telemetry on signal change ───────────────────────

/// TS-02-20: CLOUD_GATEWAY_CLIENT publishes telemetry on signal change (02-REQ-5.2)
#[tokio::test]
async fn test_cgc_publishes_telemetry() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let (sub_client, mut sub_loop) = create_test_mqtt("test-sub-s20");
    mqtt_connect(&mut sub_loop).await;

    sub_client
        .subscribe("vehicles/VIN12345/telemetry", QoS::AtLeastOnce)
        .await
        .unwrap();

    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match sub_loop.poll().await {
                Ok(Event::Incoming(Packet::SubAck(_))) => break,
                Ok(_) => continue,
                Err(e) => panic!("MQTT error: {}", e),
            }
        }
    })
    .await;

    tokio::time::sleep(Duration::from_millis(500)).await;

    db_client
        .set_value("Vehicle.CurrentLocation.Latitude", DataValue::Double(48.1351))
        .await
        .unwrap();

    let msg1 = mqtt_wait_for_message(&mut sub_loop, Duration::from_secs(10)).await;
    assert!(msg1.is_some(), "should receive telemetry for location change");

    set_speed(&db_client, 60.0).await;
    let msg2 = mqtt_wait_for_message(&mut sub_loop, Duration::from_secs(10)).await;
    assert!(msg2.is_some(), "should receive telemetry for speed change");

    handle.abort();
}
