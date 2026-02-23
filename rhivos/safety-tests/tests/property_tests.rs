//! Property tests for RHIVOS safety partition
//!
//! These tests verify correctness properties (invariants) of the safety
//! partition services. Each test corresponds to a property defined in design.md.
//!
//! Test Spec: TS-02-P1 through TS-02-P8

use std::time::Duration;

use databroker_client::{DataValue, DatabrokerClient};
use locking_service::command::{self, LockAction, ParseResult};
use locking_service::safety::SafetyChecker;
use locking_service::service::signals;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use tokio_stream::StreamExt;

/// Helper: check if DATA_BROKER infrastructure is available.
fn infra_available() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:55556".parse().unwrap(),
        Duration::from_secs(2),
    )
    .is_ok()
}

/// Helper: check if MQTT broker is available.
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
    };
}

macro_rules! require_full_infra {
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
async fn test_client() -> DatabrokerClient {
    DatabrokerClient::connect("http://localhost:55556")
        .await
        .expect("should connect to DATA_BROKER on port 55556")
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
async fn set_door_open(client: &DatabrokerClient, is_open: bool) {
    client
        .set_value(
            "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
            DataValue::Bool(is_open),
        )
        .await
        .expect("should set IsOpen");
}

/// Read the current lock state from DATA_BROKER.
async fn get_locked(client: &DatabrokerClient) -> Option<bool> {
    client
        .get_value_opt(signals::IS_LOCKED)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.as_bool())
}

/// Process a command (mirrors the locking-service logic for in-process testing).
async fn process_command_for_test(
    payload: &str,
    client: &DatabrokerClient,
    safety: &SafetyChecker,
) -> locking_service::command::CommandResponse {
    use locking_service::command::{reason, CommandResponse};

    let cmd = match command::parse_command(payload) {
        ParseResult::Ok(cmd) => cmd,
        ParseResult::InvalidPayload => {
            return CommandResponse::failed_no_id(reason::INVALID_PAYLOAD);
        }
        ParseResult::MissingFields => {
            return CommandResponse::failed_no_id(reason::MISSING_FIELDS);
        }
        ParseResult::UnknownAction { command_id } => {
            let id = command_id.as_deref().unwrap_or("");
            return CommandResponse::failed(id, reason::UNKNOWN_ACTION);
        }
    };

    let constraint_result = match cmd.action {
        LockAction::Lock => safety.check_lock_constraints().await,
        LockAction::Unlock => safety.check_unlock_constraints().await,
    };

    if let Err(reason) = constraint_result {
        return CommandResponse::failed(&cmd.command_id, &reason);
    }

    let lock_value = matches!(cmd.action, LockAction::Lock);
    if client
        .set_value(signals::IS_LOCKED, DataValue::Bool(lock_value))
        .await
        .is_err()
    {
        return CommandResponse::failed(&cmd.command_id, "write_failed");
    }

    CommandResponse::success(&cmd.command_id)
}

/// Spawn the locking service processing loop in a background task.
fn spawn_locking_service(client: DatabrokerClient) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let safety = SafetyChecker::new(client.clone());
        let mut stream = client
            .subscribe(&[signals::COMMAND])
            .await
            .expect("should subscribe to command signal");

        while let Some(Ok(updates)) = stream.next().await {
            for update in updates {
                if update.path != signals::COMMAND {
                    continue;
                }
                let payload = match &update.value {
                    Some(DataValue::String(s)) => s.clone(),
                    _ => continue,
                };

                let response = process_command_for_test(&payload, &client, &safety).await;
                let response_json = response.to_json();
                let _ = client
                    .set_value(signals::RESPONSE, DataValue::String(response_json))
                    .await;
            }
        }
    })
}

/// Send a command and wait for the response.
async fn send_command_and_wait(
    client: &DatabrokerClient,
    command_json: &str,
    timeout: Duration,
) -> Option<serde_json::Value> {
    let mut stream = client
        .subscribe(&[signals::RESPONSE])
        .await
        .expect("should subscribe to response signal");

    client
        .set_value(
            signals::COMMAND,
            DataValue::String(command_json.to_string()),
        )
        .await
        .expect("should write command signal");

    let result = tokio::time::timeout(timeout, stream.next()).await;

    match result {
        Ok(Some(Ok(updates))) => {
            for update in updates {
                if update.path == signals::RESPONSE {
                    if let Some(DataValue::String(json)) = update.value {
                        return serde_json::from_str(&json).ok();
                    }
                }
            }
            None
        }
        _ => None,
    }
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

/// Helper: find the workspace target directory and return the sensor binary path.
fn sensor_binary(name: &str) -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR should be set by cargo");
    let workspace_root = std::path::PathBuf::from(&manifest_dir)
        .parent()
        .expect("safety-tests should be inside workspace")
        .to_path_buf();
    let binary = workspace_root.join("target").join("debug").join(name);

    if !binary.exists() {
        let build_result = std::process::Command::new("cargo")
            .args(["build", "--bin", name, "-p", "mock-sensors"])
            .current_dir(&workspace_root)
            .output();

        match build_result {
            Ok(output) if !output.status.success() => {
                panic!(
                    "could not build {}: cargo build failed: {}",
                    name,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(e) => {
                panic!("could not build {}: {}", name, e);
            }
            _ => {}
        }
    }

    assert!(binary.exists(), "{} binary not found at {}", name, binary.display());
    binary
}

// ── TS-02-P1: Command-Response Pairing ──────────────────────────────────────

/// TS-02-P1: Command-Response Pairing — Property 1
///
/// For any lock/unlock command written to Vehicle.Command.Door.Lock, the
/// LOCKING_SERVICE SHALL eventually write exactly one Vehicle.Command.Door.Response
/// with the same command_id.
///
/// Validates: 02-REQ-2.2, 02-REQ-3.4, 02-REQ-3.5
#[tokio::test]
async fn test_property_command_response_pairing() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    set_speed(&client, 0.0).await;
    set_door_open(&client, false).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    for i in 0..5 {
        let cmd_id = format!("pairing-p-{}", i);
        let response = send_command_and_wait(
            &client,
            &make_command(&cmd_id, "lock"),
            Duration::from_secs(5),
        )
        .await;

        let resp = response.unwrap_or_else(|| panic!("should receive response for command {}", i));
        assert_eq!(
            resp["command_id"], cmd_id,
            "response command_id should match sent command_id"
        );
    }

    service_handle.abort();
}

// ── TS-02-P2: Safety Constraint Enforcement (Speed) ─────────────────────────

/// TS-02-P2: Safety Constraint Enforcement (Speed) — Property 2
///
/// For any lock command received when Vehicle.Speed > 0, the LOCKING_SERVICE
/// SHALL NOT change IsLocked, AND the response SHALL have status "failed".
///
/// Validates: 02-REQ-3.1, 02-REQ-3.4
#[tokio::test]
async fn test_property_safety_constraint_speed() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    set_door_open(&client, false).await;

    for speed in [1.0_f32, 10.0, 100.0, 0.1] {
        let initial = get_locked(&client).await;

        set_speed(&client, speed).await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        let response = send_command_and_wait(
            &client,
            &make_command(&format!("speed-p-{}", speed), "lock"),
            Duration::from_secs(5),
        )
        .await;

        let resp = response.unwrap_or_else(|| panic!("should get response for speed={}", speed));
        assert_eq!(resp["status"], "failed", "should fail at speed={}", speed);
        assert_eq!(resp["reason"], "vehicle_moving");

        let current = get_locked(&client).await;
        assert_eq!(current, initial, "IsLocked should not change at speed={}", speed);
    }

    service_handle.abort();
}

// ── TS-02-P3: Door Ajar Protection ──────────────────────────────────────────

/// TS-02-P3: Door Ajar Protection — Property 3
///
/// For any lock command received when IsOpen == true, the LOCKING_SERVICE SHALL
/// NOT change IsLocked, AND the response SHALL have status "failed" with reason
/// "door_open".
///
/// Validates: 02-REQ-3.2, 02-REQ-3.4
#[tokio::test]
async fn test_property_door_ajar_protection() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    set_speed(&client, 0.0).await;
    set_door_open(&client, true).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let initial = get_locked(&client).await;

    let response = send_command_and_wait(
        &client,
        &make_command("ajar-p-test", "lock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "failed");
    assert_eq!(resp["reason"], "door_open");

    let current = get_locked(&client).await;
    assert_eq!(current, initial, "IsLocked should not change when door is open");
}

// ── TS-02-P4: Lock State Consistency ────────────────────────────────────────

/// TS-02-P4: Lock State Consistency — Property 4
///
/// For any successful lock/unlock command, after the LOCKING_SERVICE writes the
/// response, IsLocked SHALL match the commanded action (true for "lock", false
/// for "unlock").
///
/// Validates: 02-REQ-3.5
#[tokio::test]
async fn test_property_lock_state_consistency() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    set_speed(&client, 0.0).await;
    set_door_open(&client, false).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Lock
    let resp = send_command_and_wait(
        &client,
        &make_command("consistency-p-1", "lock"),
        Duration::from_secs(5),
    )
    .await
    .expect("should get lock response");
    assert_eq!(resp["status"], "success");
    assert_eq!(get_locked(&client).await, Some(true));

    // Unlock
    let resp = send_command_and_wait(
        &client,
        &make_command("consistency-p-2", "unlock"),
        Duration::from_secs(5),
    )
    .await
    .expect("should get unlock response");
    assert_eq!(resp["status"], "success");
    assert_eq!(get_locked(&client).await, Some(false));

    // Lock again
    let resp = send_command_and_wait(
        &client,
        &make_command("consistency-p-3", "lock"),
        Duration::from_secs(5),
    )
    .await
    .expect("should get second lock response");
    assert_eq!(resp["status"], "success");
    assert_eq!(get_locked(&client).await, Some(true));

    service_handle.abort();
}

// ── TS-02-P5: MQTT Command Relay Integrity ──────────────────────────────────

/// TS-02-P5: MQTT Command Relay Integrity — Property 5
///
/// For any valid command received on MQTT topic `vehicles/{vin}/commands`,
/// the CLOUD_GATEWAY_CLIENT SHALL write an identical command payload to
/// Vehicle.Command.Door.Lock in DATA_BROKER.
///
/// Validates: 02-REQ-4.3
#[tokio::test]
async fn test_property_mqtt_relay_integrity() {
    require_full_infra!();

    let db_client = test_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let (pub_client, mut pub_loop) = create_test_mqtt("test-pub-p5-s");
    mqtt_connect(&mut pub_loop).await;

    let commands = vec![
        make_command("p5-s-1", "lock"),
        make_command("p5-s-2", "unlock"),
    ];

    for cmd in &commands {
        pub_client
            .publish("vehicles/VIN12345/commands", QoS::AtLeastOnce, false, cmd.as_bytes())
            .await
            .unwrap();

        // Drive the pub loop
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

        if let Some(DataValue::String(v)) = value {
            let sent: serde_json::Value = serde_json::from_str(cmd).unwrap();
            let received: serde_json::Value = serde_json::from_str(&v).unwrap();
            assert_eq!(received["command_id"], sent["command_id"], "command_id must match");
            assert_eq!(received["action"], sent["action"], "action must match");
        } else {
            panic!("expected command in DATA_BROKER");
        }
    }

    handle.abort();
}

// ── TS-02-P6: Telemetry Signal Coverage ─────────────────────────────────────

/// TS-02-P6: Telemetry Signal Coverage — Property 6
///
/// For any change to a subscribed vehicle state signal in DATA_BROKER, the
/// CLOUD_GATEWAY_CLIENT SHALL publish a telemetry message to MQTT containing
/// the signal path, new value, and timestamp.
///
/// Validates: 02-REQ-5.1, 02-REQ-5.2
#[tokio::test]
async fn test_property_telemetry_coverage() {
    require_full_infra!();

    let db_client = test_client().await;
    let handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let (sub_client, mut sub_loop) = create_test_mqtt("test-sub-p6-s");
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

    set_speed(&db_client, 99.0).await;

    let msg = mqtt_wait_for_message(&mut sub_loop, Duration::from_secs(10)).await;

    handle.abort();

    let (_topic, payload) = msg.expect("should receive telemetry for speed change");
    let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    let signals = parsed["signals"].as_array().expect("should have signals array");
    assert!(
        signals.iter().any(|s| s["path"].as_str().unwrap_or("").contains("Speed")),
        "telemetry should contain Speed signal"
    );
}

/// Helper: create a test MQTT client.
fn create_test_mqtt(client_id: &str) -> (AsyncClient, rumqttc::EventLoop) {
    let mut opts = MqttOptions::new(client_id, "localhost", 1883);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_clean_session(true);
    AsyncClient::new(opts, 64)
}

// ── TS-02-P8: Sensor Idempotency ───────────────────────────────────────────

/// TS-02-P8: Sensor Idempotency — Property 8
///
/// For any mock sensor CLI invocation with the same arguments, the resulting
/// signal value in DATA_BROKER SHALL be identical regardless of the number of
/// prior invocations.
///
/// Validates: 02-REQ-6.1 through 02-REQ-6.4
#[tokio::test]
async fn test_property_sensor_idempotency() {
    require_infra!();

    let binary = sensor_binary("speed-sensor");
    let client = test_client().await;

    // Run speed-sensor --speed 77.7 twice and verify idempotent behavior
    let output1 = std::process::Command::new(&binary)
        .args(["--speed", "77.7"])
        .env("DATABROKER_ADDR", "http://localhost:55556")
        .output()
        .expect("should run speed-sensor");
    assert!(output1.status.success(), "first invocation should succeed");

    let val1 = client
        .get_value("Vehicle.Speed")
        .await
        .expect("should read Vehicle.Speed after first write");

    // Second invocation with same value
    let output2 = std::process::Command::new(&binary)
        .args(["--speed", "77.7"])
        .env("DATABROKER_ADDR", "http://localhost:55556")
        .output()
        .expect("should run speed-sensor again");
    assert!(output2.status.success(), "second invocation should succeed");

    let val2 = client
        .get_value("Vehicle.Speed")
        .await
        .expect("should read Vehicle.Speed after second write");

    // Both reads should return the same value
    let v1 = val1.as_float().expect("Vehicle.Speed should be a float");
    let v2 = val2.as_float().expect("Vehicle.Speed should be a float");
    assert!(
        (v1 - v2).abs() < 0.01,
        "idempotent writes should produce same value: {} vs {}",
        v1,
        v2
    );
    assert!(
        (v1 - 77.7).abs() < 0.5,
        "value should be approximately 77.7, got {}",
        v1
    );
}
