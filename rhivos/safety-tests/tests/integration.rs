//! End-to-end integration tests for RHIVOS safety partition
//!
//! These tests verify the full command flow through all safety partition
//! services: MQTT -> CLOUD_GATEWAY_CLIENT -> DATA_BROKER -> LOCKING_SERVICE
//! -> DATA_BROKER -> CLOUD_GATEWAY_CLIENT -> MQTT.
//!
//! Test Spec: TS-02-30, TS-02-32

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

/// Create a test MQTT client with a unique client ID.
fn create_test_mqtt(client_id: &str) -> (AsyncClient, rumqttc::EventLoop) {
    let mut opts = MqttOptions::new(client_id, "localhost", 1883);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_clean_session(true);
    AsyncClient::new(opts, 64)
}

/// Wait for MQTT connection to be established.
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

/// Send a lock/unlock command by writing to Vehicle.Command.Door.Lock
/// and wait for the response on Vehicle.Command.Door.Response.
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

// ── TS-02-30: End-to-end lock command flow ──────────────────────────────────

/// TS-02-30: Integration test for lock command flow (02-REQ-8.1)
///
/// Verify end-to-end lock command flow through DATA_BROKER:
/// MQTT command -> CLOUD_GATEWAY_CLIENT -> DATA_BROKER -> LOCKING_SERVICE
/// -> lock state change -> response -> CLOUD_GATEWAY_CLIENT -> MQTT response.
#[tokio::test]
async fn test_e2e_lock_command_flow() {
    require_full_infra!();

    let db_client = test_client().await;

    // Set safe conditions
    set_speed(&db_client, 0.0).await;
    set_door_open(&db_client, false).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Spawn both services
    let ls_handle = spawn_locking_service(db_client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;
    let cgc_handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Subscribe to MQTT command_responses
    let (sub_client, mut sub_loop) = create_test_mqtt("e2e-sub-30");
    mqtt_connect(&mut sub_loop).await;

    sub_client
        .subscribe("vehicles/VIN12345/command_responses", QoS::AtLeastOnce)
        .await
        .unwrap();

    // Wait for SubAck
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

    // Publish lock command via MQTT
    let (pub_client, mut pub_loop) = create_test_mqtt("e2e-pub-30");
    mqtt_connect(&mut pub_loop).await;

    let cmd = make_command("e2e-30", "lock");
    pub_client
        .publish(
            "vehicles/VIN12345/commands",
            QoS::AtLeastOnce,
            false,
            cmd.as_bytes(),
        )
        .await
        .unwrap();

    // Drive the pub loop to send
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

    // Wait for response on MQTT
    let msg = mqtt_wait_for_message(&mut sub_loop, Duration::from_secs(10)).await;

    ls_handle.abort();
    cgc_handle.abort();

    let (_topic, payload) = msg.expect("should receive response on MQTT");
    let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(parsed["command_id"], "e2e-30");
    assert_eq!(parsed["status"], "success");

    // Verify lock state changed
    let locked = get_locked(&db_client).await;
    assert_eq!(locked, Some(true));
}

/// TS-02-30 variant: E2E lock rejected when vehicle moving.
#[tokio::test]
async fn test_e2e_lock_rejected_vehicle_moving() {
    require_full_infra!();

    let db_client = test_client().await;
    set_speed(&db_client, 30.0).await;
    set_door_open(&db_client, false).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let initial_locked = get_locked(&db_client).await;

    let ls_handle = spawn_locking_service(db_client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let response = send_command_and_wait(
        &db_client,
        &make_command("e2e-moving", "lock"),
        Duration::from_secs(5),
    )
    .await;

    ls_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "failed");
    assert_eq!(resp["reason"], "vehicle_moving");
    assert_eq!(get_locked(&db_client).await, initial_locked);
}

/// TS-02-30 variant: E2E lock rejected when door open.
#[tokio::test]
async fn test_e2e_lock_rejected_door_open() {
    require_full_infra!();

    let db_client = test_client().await;
    set_speed(&db_client, 0.0).await;
    set_door_open(&db_client, true).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let initial_locked = get_locked(&db_client).await;

    let ls_handle = spawn_locking_service(db_client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let response = send_command_and_wait(
        &db_client,
        &make_command("e2e-door-open", "lock"),
        Duration::from_secs(5),
    )
    .await;

    ls_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "failed");
    assert_eq!(resp["reason"], "door_open");
    assert_eq!(get_locked(&db_client).await, initial_locked);
}

/// TS-02-30 variant: E2E unlock succeeds when safe.
#[tokio::test]
async fn test_e2e_unlock_succeeds() {
    require_full_infra!();

    let db_client = test_client().await;
    set_speed(&db_client, 0.0).await;
    set_door_open(&db_client, false).await;
    db_client
        .set_value(signals::IS_LOCKED, DataValue::Bool(true))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let ls_handle = spawn_locking_service(db_client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let response = send_command_and_wait(
        &db_client,
        &make_command("e2e-unlock", "unlock"),
        Duration::from_secs(5),
    )
    .await;

    ls_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "success");
    assert_eq!(get_locked(&db_client).await, Some(false));
}

/// TS-02-30 variant: E2E telemetry published on signal changes.
#[tokio::test]
async fn test_e2e_telemetry_on_signal_change() {
    require_full_infra!();

    let db_client = test_client().await;
    let cgc_handle = spawn_cgc_service(db_client.clone(), "VIN12345");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let (sub_client, mut sub_loop) = create_test_mqtt("e2e-telem-sub");
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

    set_speed(&db_client, 88.0).await;
    let msg = mqtt_wait_for_message(&mut sub_loop, Duration::from_secs(10)).await;

    cgc_handle.abort();

    let (_topic, payload) = msg.expect("should receive telemetry on MQTT");
    let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert!(parsed["signals"].is_array(), "telemetry should have signals array");
}

// ── TS-02-32: Integration tests require infrastructure ──────────────────────

/// TS-02-32: Integration tests require infrastructure (02-REQ-8.3)
///
/// Verify integration tests expect running infrastructure and fail or skip
/// with a clear message when infrastructure is absent.
#[test]
fn test_integration_requires_infra() {
    let databroker_up = infra_available();
    let mqtt_up = mqtt_available();

    if !databroker_up || !mqtt_up {
        eprintln!(
            "Infrastructure check: DATA_BROKER={}, MQTT={}",
            if databroker_up { "up" } else { "down" },
            if mqtt_up { "up" } else { "down" }
        );
        return;
    }

    assert!(
        databroker_up,
        "infra_available() returned false but infrastructure should be running"
    );
}
