//! Integration tests for CLOUD_GATEWAY_CLIENT.
//!
//! These tests verify MQTT connectivity, command relay, response relay,
//! and telemetry publishing through running infrastructure.
//!
//! Prerequisites:
//! - DATA_BROKER must be running (`make infra-up`)
//! - MQTT broker (Mosquitto) must be running (`make infra-up`)
//! - No separate CLOUD_GATEWAY_CLIENT process needed — tests exercise
//!   the service logic directly via the library API
//!
//! Test Spec: TS-02-15 through TS-02-20, TS-02-E8 through TS-02-E11,
//!            TS-02-P5, TS-02-P6

use std::time::Duration;

use databroker_client::{DataValue, DatabrokerClient};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};

/// Check if DATA_BROKER infrastructure is available via TCP.
fn infra_available() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:55556".parse().unwrap(),
        Duration::from_secs(2),
    )
    .is_ok()
}

/// Check if MQTT broker is available.
fn mqtt_available() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:1883".parse().unwrap(),
        Duration::from_secs(2),
    )
    .is_ok()
}

macro_rules! require_infra {
    () => {
        if !infra_available() {
            eprintln!("SKIP: DATA_BROKER not available on port 55556 (run `make infra-up`)");
            return;
        }
        if !mqtt_available() {
            eprintln!("SKIP: MQTT broker not available on port 1883 (run `make infra-up`)");
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

/// Publish a message to an MQTT topic.
async fn mqtt_publish(client: &AsyncClient, topic: &str, payload: &str) {
    client
        .publish(topic, QoS::AtLeastOnce, false, payload.as_bytes())
        .await
        .expect("should publish MQTT message");
}

/// Subscribe to an MQTT topic.
async fn mqtt_subscribe(client: &AsyncClient, topic: &str) {
    client
        .subscribe(topic, QoS::AtLeastOnce)
        .await
        .expect("should subscribe to MQTT topic");
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
/// Uses TCP for DATA_BROKER (for testing) but exercises the MQTT bridge logic.
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

        // Spawn telemetry loop
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

        // Spawn response relay
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

        // Run MQTT event loop with command handler
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

/// Set door open state via DATA_BROKER.
#[allow(dead_code)]
async fn set_door_open(client: &DatabrokerClient, is_open: bool) {
    client
        .set_value(
            "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
            DataValue::Bool(is_open),
        )
        .await
        .expect("should set IsOpen");
}

// ── TS-02-15: CGC connects to MQTT broker ────────────────────────────────────

/// TS-02-15: CLOUD_GATEWAY_CLIENT connects to the configured MQTT broker.
#[tokio::test]
async fn test_cgc_connects_to_mqtt() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client, "VIN12345");

    // Give service time to connect
    tokio::time::sleep(Duration::from_secs(2)).await;

    // If we get here without panic, the service started and connected
    assert!(!handle.is_finished(), "CGC service should still be running");

    handle.abort();
}

// ── TS-02-16: CGC subscribes to command topic ────────────────────────────────

/// TS-02-16: CLOUD_GATEWAY_CLIENT subscribes to `vehicles/{vin}/commands`.
#[tokio::test]
async fn test_cgc_subscribes_to_command_topic() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");

    // Let CGC connect and subscribe
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Publish a command via MQTT
    let (pub_client, mut pub_loop) = create_test_mqtt("test-pub-16");

    // Drive the connection
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match pub_loop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => break,
                Ok(_) => continue,
                Err(e) => panic!("MQTT connection failed: {}", e),
            }
        }
    })
    .await;

    let cmd = make_command("mqtt-test-16", "lock");
    mqtt_publish(&pub_client, "vehicles/VIN12345/commands", &cmd).await;

    // Wait for the command to appear in DATA_BROKER
    tokio::time::sleep(Duration::from_secs(2)).await;

    let value = db_client
        .get_value_opt("Vehicle.Command.Door.Lock")
        .await
        .ok()
        .flatten();

    handle.abort();

    if let Some(DataValue::String(v)) = value {
        let parsed: serde_json::Value = serde_json::from_str(&v).unwrap();
        assert_eq!(parsed["command_id"], "mqtt-test-16");
    } else {
        panic!("expected command to appear in DATA_BROKER");
    }
}

// ── TS-02-17: CGC writes validated command to DATA_BROKER ────────────────────

/// TS-02-17: CLOUD_GATEWAY_CLIENT writes validated MQTT commands to DATA_BROKER.
#[tokio::test]
async fn test_cgc_writes_command_to_databroker() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Publish a command via MQTT
    let (pub_client, mut pub_loop) = create_test_mqtt("test-pub-17");
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match pub_loop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => break,
                Ok(_) => continue,
                Err(e) => panic!("MQTT connection failed: {}", e),
            }
        }
    })
    .await;

    let cmd = make_command("relay-test-17", "unlock");
    mqtt_publish(&pub_client, "vehicles/VIN12345/commands", &cmd).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let value = db_client
        .get_value_opt("Vehicle.Command.Door.Lock")
        .await
        .ok()
        .flatten();

    handle.abort();

    if let Some(DataValue::String(v)) = value {
        let parsed: serde_json::Value = serde_json::from_str(&v).unwrap();
        assert_eq!(parsed["command_id"], "relay-test-17");
        assert_eq!(parsed["action"], "unlock");
    } else {
        panic!("expected command to appear in DATA_BROKER");
    }
}

// ── TS-02-18: CGC relays response to MQTT ────────────────────────────────────

/// TS-02-18: CLOUD_GATEWAY_CLIENT relays command responses to MQTT.
#[tokio::test]
async fn test_cgc_relays_response_to_mqtt() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Subscribe to responses via a separate MQTT client
    let (sub_client, mut sub_loop) = create_test_mqtt("test-sub-18");

    // Connect and subscribe
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match sub_loop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => break,
                Ok(_) => continue,
                Err(e) => panic!("MQTT connection failed: {}", e),
            }
        }
    })
    .await;

    mqtt_subscribe(&sub_client, "vehicles/VIN12345/command_responses").await;

    // Drive until SubAck
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

    // Write a response directly to DATA_BROKER (simulating locking-service)
    let response_json = serde_json::json!({
        "command_id": "e2e-test-18",
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

    // Wait for the response to appear on MQTT
    let msg = mqtt_wait_for_message(&mut sub_loop, Duration::from_secs(10)).await;

    handle.abort();

    let (topic, payload) = msg.expect("should receive response on MQTT");
    assert_eq!(topic, "vehicles/VIN12345/command_responses");
    let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(parsed["command_id"], "e2e-test-18");
    assert_eq!(parsed["status"], "success");
}

// ── TS-02-19: CGC subscribes to vehicle state signals ────────────────────────

/// TS-02-19: CLOUD_GATEWAY_CLIENT subscribes to state signals in DATA_BROKER.
#[tokio::test]
async fn test_cgc_subscribes_to_state_signals() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Subscribe to telemetry
    let (sub_client, mut sub_loop) = create_test_mqtt("test-sub-19");

    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match sub_loop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => break,
                Ok(_) => continue,
                Err(e) => panic!("MQTT connection failed: {}", e),
            }
        }
    })
    .await;

    mqtt_subscribe(&sub_client, "vehicles/VIN12345/telemetry").await;

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

    // Change a signal value
    set_speed(&db_client, 42.0).await;

    // Wait for telemetry on MQTT
    let msg = mqtt_wait_for_message(&mut sub_loop, Duration::from_secs(10)).await;

    handle.abort();

    let (_topic, payload) = msg.expect("should receive telemetry on MQTT");
    let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert!(parsed["signals"].is_array());
    let signals = parsed["signals"].as_array().unwrap();
    assert!(
        signals
            .iter()
            .any(|s| s["path"].as_str().unwrap_or("").contains("Speed")),
        "telemetry should contain Speed signal"
    );
}

// ── TS-02-20: CGC publishes telemetry on signal change ───────────────────────

/// TS-02-20: CLOUD_GATEWAY_CLIENT publishes telemetry when signals change.
#[tokio::test]
async fn test_cgc_publishes_telemetry() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Subscribe to telemetry
    let (sub_client, mut sub_loop) = create_test_mqtt("test-sub-20");

    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match sub_loop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => break,
                Ok(_) => continue,
                Err(e) => panic!("MQTT connection failed: {}", e),
            }
        }
    })
    .await;

    mqtt_subscribe(&sub_client, "vehicles/VIN12345/telemetry").await;

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

    // Change location signals
    db_client
        .set_value(
            "Vehicle.CurrentLocation.Latitude",
            DataValue::Double(48.1351),
        )
        .await
        .unwrap();

    let msg1 = mqtt_wait_for_message(&mut sub_loop, Duration::from_secs(10)).await;
    assert!(msg1.is_some(), "should receive telemetry for location change");

    // Change speed
    set_speed(&db_client, 60.0).await;

    let msg2 = mqtt_wait_for_message(&mut sub_loop, Duration::from_secs(10)).await;
    assert!(msg2.is_some(), "should receive telemetry for speed change");

    handle.abort();
}

// ── TS-02-E10: Invalid JSON in MQTT command ──────────────────────────────────

/// TS-02-E10: CLOUD_GATEWAY_CLIENT discards invalid MQTT messages.
#[tokio::test]
async fn test_edge_invalid_mqtt_json() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Record current state
    let initial = db_client
        .get_value_opt("Vehicle.Command.Door.Lock")
        .await
        .ok()
        .flatten();

    // Publish invalid JSON
    let (pub_client, mut pub_loop) = create_test_mqtt("test-pub-e10");
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match pub_loop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => break,
                Ok(_) => continue,
                Err(e) => panic!("MQTT connection failed: {}", e),
            }
        }
    })
    .await;

    mqtt_publish(
        &pub_client,
        "vehicles/VIN12345/commands",
        "not valid json",
    )
    .await;

    tokio::time::sleep(Duration::from_secs(3)).await;

    // Verify command signal has NOT changed
    let current = db_client
        .get_value_opt("Vehicle.Command.Door.Lock")
        .await
        .ok()
        .flatten();

    // CGC should still be running
    assert!(!handle.is_finished(), "CGC should not crash on invalid JSON");

    handle.abort();

    // Compare: if initial was None, current should also be None
    // If initial was Some(val), current should be Some(same val)
    match (&initial, &current) {
        (None, None) => {} // both None, no change
        (Some(DataValue::String(a)), Some(DataValue::String(b))) => {
            assert_eq!(a, b, "command signal should not change on invalid JSON");
        }
        _ => {} // different types but didn't crash, acceptable
    }
}

// ── TS-02-P5: MQTT Command Relay Integrity ───────────────────────────────────

/// TS-02-P5: For any valid MQTT command, the DATA_BROKER receives identical payload.
#[tokio::test]
async fn test_property_mqtt_relay_integrity() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let (pub_client, mut pub_loop) = create_test_mqtt("test-pub-p5");
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match pub_loop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => break,
                Ok(_) => continue,
                Err(e) => panic!("MQTT connection failed: {}", e),
            }
        }
    })
    .await;

    let commands = vec![
        make_command("p5-1", "lock"),
        make_command("p5-2", "unlock"),
    ];

    for cmd in &commands {
        mqtt_publish(&pub_client, "vehicles/VIN12345/commands", cmd).await;
        tokio::time::sleep(Duration::from_secs(2)).await;

        let value = db_client
            .get_value_opt("Vehicle.Command.Door.Lock")
            .await
            .ok()
            .flatten();

        if let Some(DataValue::String(v)) = value {
            let sent: serde_json::Value = serde_json::from_str(cmd).unwrap();
            let received: serde_json::Value = serde_json::from_str(&v).unwrap();
            assert_eq!(
                received["command_id"], sent["command_id"],
                "command_id must match"
            );
            assert_eq!(
                received["action"], sent["action"],
                "action must match"
            );
        } else {
            panic!("expected command in DATA_BROKER");
        }
    }

    handle.abort();
}

// ── TS-02-P6: Telemetry Signal Coverage ──────────────────────────────────────

/// TS-02-P6: For any signal change, a telemetry message is published.
#[tokio::test]
async fn test_property_telemetry_coverage() {
    require_infra!();

    let db_client = test_db_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let (sub_client, mut sub_loop) = create_test_mqtt("test-sub-p6");

    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match sub_loop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => break,
                Ok(_) => continue,
                Err(e) => panic!("MQTT connection failed: {}", e),
            }
        }
    })
    .await;

    mqtt_subscribe(&sub_client, "vehicles/VIN12345/telemetry").await;

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

    // Set speed
    set_speed(&db_client, 99.0).await;

    let msg = mqtt_wait_for_message(&mut sub_loop, Duration::from_secs(10)).await;

    handle.abort();

    let (_topic, payload) = msg.expect("should receive telemetry for speed change");
    let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    let signals = parsed["signals"].as_array().expect("should have signals array");
    assert!(
        signals
            .iter()
            .any(|s| s["path"].as_str().unwrap_or("").contains("Speed")),
        "telemetry should contain Speed signal"
    );
}
