//! Integration tests for CLOUD_GATEWAY_CLIENT.
//!
//! These tests require a running NATS server and DATA_BROKER (Kuksa Databroker)
//! and are gated behind the `integration` feature flag.
//!
//! Run with: `cd rhivos && cargo test -p cloud-gateway-client --features integration`
//! Prerequisites: `make infra-up`

#![cfg(feature = "integration")]

use std::time::Duration;

use bytes::Bytes;
use cloud_gateway_client::command::Command;
use cloud_gateway_client::config::Config;
use cloud_gateway_client::databroker_client::{DataBrokerClient, SignalValue};
use cloud_gateway_client::nats_client::NatsClient;
use futures::StreamExt;

/// Default TCP address for DATA_BROKER in the test environment.
const DATABROKER_ADDR: &str = "127.0.0.1:55556";

/// NATS URL for the test environment.
const NATS_URL: &str = "nats://localhost:4222";

/// Signal paths used in tests.
const DOOR_LOCK_SIGNAL: &str = "Vehicle.Command.Door.Lock";
const DOOR_RESPONSE_SIGNAL: &str = "Vehicle.Command.Door.Response";
const IS_LOCKED_SIGNAL: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
const LATITUDE_SIGNAL: &str = "Vehicle.CurrentLocation.Latitude";
const LONGITUDE_SIGNAL: &str = "Vehicle.CurrentLocation.Longitude";
const PARKING_SIGNAL: &str = "Vehicle.Parking.SessionActive";

/// Helper: connect to DATA_BROKER via TCP.
async fn connect_databroker() -> DataBrokerClient {
    DataBrokerClient::connect_tcp(DATABROKER_ADDR)
        .await
        .expect("Failed to connect to DATA_BROKER. Is infra running? (make infra-up)")
}

/// Helper: connect to NATS with a given VIN.
async fn connect_nats(vin: &str) -> NatsClient {
    let config = Config {
        vin: vin.to_string(),
        nats_url: NATS_URL.to_string(),
        nats_tls_enabled: false,
        databroker_uds_path: "/tmp/kuksa/databroker.sock".to_string(),
    };
    NatsClient::connect(&config)
        .await
        .expect("Failed to connect to NATS. Is infra running? (make infra-up)")
}

/// Helper: create a raw NATS client for publishing/subscribing independently.
async fn connect_raw_nats() -> async_nats::Client {
    async_nats::connect(NATS_URL)
        .await
        .expect("Failed to connect to NATS")
}

/// Helper: build a valid command JSON string.
fn valid_command_json(command_id: &str, action: &str, vin: &str) -> String {
    serde_json::json!({
        "command_id": command_id,
        "action": action,
        "doors": ["driver"],
        "source": "companion_app",
        "vin": vin,
        "timestamp": 1700000000u64
    })
    .to_string()
}

/// TS-04-1: NATS Connection and Command Subscription.
///
/// Verify that the CLOUD_GATEWAY_CLIENT connects to NATS and subscribes
/// to `vehicles.{VIN}.commands`.
#[tokio::test]
async fn test_nats_connection_and_command_subscription() {
    let vin = "TEST_VIN_TS04_1";
    let nats_client = connect_nats(vin).await;

    // Subscribe to commands
    let mut sub = nats_client.subscribe_commands().await.expect("subscribe");

    // Publish a test message on the commands subject
    let raw_nats = connect_raw_nats().await;
    let subject = format!("vehicles.{}.commands", vin);
    let payload = valid_command_json("550e8400-e29b-41d4-a716-446655440000", "lock", vin);
    raw_nats
        .publish(subject, Bytes::from(payload.clone()))
        .await
        .expect("publish");
    raw_nats.flush().await.expect("flush");

    // Verify we receive the message
    let msg = tokio::time::timeout(Duration::from_secs(5), sub.next())
        .await
        .expect("timeout waiting for message")
        .expect("subscription ended");

    let received = std::str::from_utf8(&msg.payload).expect("utf8");
    let cmd: Command = serde_json::from_str(received).expect("parse");
    assert_eq!(cmd.command_id, "550e8400-e29b-41d4-a716-446655440000");
    assert_eq!(cmd.action, "lock");
}

/// TS-04-P1: Command Reception and DATA_BROKER Write.
///
/// Publish a valid command via NATS, process it through the command processor,
/// and verify it is written to `Vehicle.Command.Door.Lock` on DATA_BROKER.
#[tokio::test]
async fn test_command_reception_and_databroker_write() {
    let vin = "TEST_VIN_TS04_P1";
    let nats_client = connect_nats(vin).await;
    let mut databroker = connect_databroker().await;

    // Subscribe to commands
    let commands_sub = nats_client.subscribe_commands().await.expect("subscribe");

    // Spawn command processor with the TCP-connected databroker
    let db_clone = databroker.clone();
    let processor_handle = tokio::spawn(async move {
        cloud_gateway_client::command_processor::run(
            commands_sub,
            db_clone,
            "unused_in_tcp_test".to_string(),
        )
        .await;
    });

    // Give the processor a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish a valid command
    let raw_nats = connect_raw_nats().await;
    let subject = format!("vehicles.{}.commands", vin);
    let cmd_json = valid_command_json("550e8400-e29b-41d4-a716-446655440000", "lock", vin);
    raw_nats
        .publish(subject, Bytes::from(cmd_json.clone()))
        .await
        .expect("publish");
    raw_nats.flush().await.expect("flush");

    // Wait for the command to be processed and written to DATA_BROKER
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Read the signal from DATA_BROKER
    let value = databroker
        .get_signal(DOOR_LOCK_SIGNAL)
        .await
        .expect("get_signal");

    match value {
        Some(SignalValue::String(s)) => {
            let parsed: serde_json::Value = serde_json::from_str(&s).expect("parse");
            assert_eq!(
                parsed["command_id"],
                "550e8400-e29b-41d4-a716-446655440000"
            );
            assert_eq!(parsed["action"], "lock");
            assert_eq!(parsed["vin"], vin);
        }
        other => panic!(
            "Expected String signal value for {}, got {:?}",
            DOOR_LOCK_SIGNAL, other
        ),
    }

    processor_handle.abort();
}

/// TS-04-P2: Command Response Relay from DATA_BROKER to NATS.
///
/// Write a response to `Vehicle.Command.Door.Response` on DATA_BROKER and
/// verify it is published to `vehicles.{VIN}.command_responses` on NATS.
#[tokio::test]
async fn test_response_relay_databroker_to_nats() {
    let vin = "TEST_VIN_TS04_P2";
    let nats_client = connect_nats(vin).await;
    let databroker = connect_databroker().await;

    // Subscribe to the command_responses subject on NATS
    let raw_nats = connect_raw_nats().await;
    let resp_subject = format!("vehicles.{}.command_responses", vin);
    let mut resp_sub = raw_nats.subscribe(resp_subject).await.expect("subscribe");

    // Spawn the response relay task
    let relay_handle = tokio::spawn(async move {
        cloud_gateway_client::response_relay::run(
            databroker,
            nats_client,
            "unused_in_tcp_test".to_string(),
        )
        .await;
    });

    // Give the relay a moment to subscribe to DATA_BROKER
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Write a response to DATA_BROKER using a unique command_id
    let mut db_writer = connect_databroker().await;
    let unique_id = "bbbbbbbb-2222-4444-8888-aaaaaaaaaaaa";
    let response_json = serde_json::json!({
        "command_id": unique_id,
        "status": "success",
        "timestamp": 1700000001u64
    })
    .to_string();

    db_writer
        .set_signal(
            DOOR_RESPONSE_SIGNAL,
            SignalValue::String(response_json.clone()),
        )
        .await
        .expect("set_signal");

    // Wait for the response to appear on NATS (may receive stale values first)
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut found = false;
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_secs(5), resp_sub.next()).await {
            Ok(Some(msg)) => {
                let received: serde_json::Value =
                    serde_json::from_slice(&msg.payload).expect("parse response");
                if received["command_id"] == unique_id {
                    assert_eq!(received["status"], "success");
                    found = true;
                    break;
                }
                // Skip stale responses from prior tests
            }
            _ => break,
        }
    }
    assert!(found, "Expected response with command_id {} on NATS", unique_id);

    relay_handle.abort();
}

/// TS-04-P3: Telemetry Publishing on Signal Change.
///
/// Write `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true` to DATA_BROKER
/// and verify telemetry is published to `vehicles.{VIN}.telemetry` on NATS.
#[tokio::test]
async fn test_telemetry_publishing_on_signal_change() {
    let vin = "TEST_VIN_TS04_P3";
    let nats_client = connect_nats(vin).await;
    let databroker = connect_databroker().await;

    // Subscribe to the telemetry subject on NATS
    let raw_nats = connect_raw_nats().await;
    let telem_subject = format!("vehicles.{}.telemetry", vin);
    let mut telem_sub = raw_nats.subscribe(telem_subject).await.expect("subscribe");

    // Spawn the telemetry task
    let telem_handle = tokio::spawn(async move {
        cloud_gateway_client::telemetry::run(
            databroker,
            nats_client,
            "unused_in_tcp_test".to_string(),
            vin.to_string(),
        )
        .await;
    });

    // Give the telemetry task a moment to subscribe
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Write a lock state change to DATA_BROKER
    let mut db_writer = connect_databroker().await;
    db_writer
        .set_signal(IS_LOCKED_SIGNAL, SignalValue::Bool(true))
        .await
        .expect("set_signal");

    // Wait for telemetry on NATS (may receive initial values for other signals first)
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut found = false;
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_secs(5), telem_sub.next()).await {
            Ok(Some(msg)) => {
                let received: serde_json::Value =
                    serde_json::from_slice(&msg.payload).expect("parse telemetry");
                if received["signal"] == IS_LOCKED_SIGNAL {
                    assert_eq!(received["value"], true);
                    assert_eq!(received["vin"], vin);
                    assert!(received["timestamp"].is_u64(), "timestamp should be present");
                    found = true;
                    break;
                }
            }
            _ => break,
        }
    }
    assert!(found, "Expected telemetry for {} on NATS", IS_LOCKED_SIGNAL);

    telem_handle.abort();
}

/// TS-04-P4: Telemetry for Multiple Signals.
///
/// Write latitude, longitude, and parking session active signals and verify
/// each produces a telemetry message on NATS.
#[tokio::test]
async fn test_telemetry_multiple_signals() {
    let vin = "TEST_VIN_TS04_P4";
    let nats_client = connect_nats(vin).await;
    let databroker = connect_databroker().await;

    // Subscribe to the telemetry subject on NATS
    let raw_nats = connect_raw_nats().await;
    let telem_subject = format!("vehicles.{}.telemetry", vin);
    let mut telem_sub = raw_nats.subscribe(telem_subject).await.expect("subscribe");

    // Spawn the telemetry task
    let telem_handle = tokio::spawn(async move {
        cloud_gateway_client::telemetry::run(
            databroker,
            nats_client,
            "unused_in_tcp_test".to_string(),
            vin.to_string(),
        )
        .await;
    });

    // Give the telemetry task a moment to subscribe
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Write multiple signal changes
    let mut db_writer = connect_databroker().await;
    db_writer
        .set_signal(LATITUDE_SIGNAL, SignalValue::Double(48.1351))
        .await
        .expect("set latitude");
    db_writer
        .set_signal(LONGITUDE_SIGNAL, SignalValue::Double(11.5820))
        .await
        .expect("set longitude");
    db_writer
        .set_signal(PARKING_SIGNAL, SignalValue::Bool(true))
        .await
        .expect("set parking");

    // Collect telemetry messages until we have our 3 target signals
    // (may also receive initial values for other subscribed signals)
    let mut received_signals = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_secs(5), telem_sub.next()).await {
            Ok(Some(msg)) => {
                let parsed: serde_json::Value =
                    serde_json::from_slice(&msg.payload).expect("parse");
                let signal = parsed["signal"].as_str().unwrap().to_string();
                if !received_signals.contains(&signal) {
                    received_signals.push(signal);
                }
                // Stop early if we have all 3 target signals
                if received_signals.contains(&LATITUDE_SIGNAL.to_string())
                    && received_signals.contains(&LONGITUDE_SIGNAL.to_string())
                    && received_signals.contains(&PARKING_SIGNAL.to_string())
                {
                    break;
                }
            }
            _ => break,
        }
    }

    assert!(
        received_signals.contains(&LATITUDE_SIGNAL.to_string()),
        "Should receive latitude telemetry, got: {:?}",
        received_signals
    );
    assert!(
        received_signals.contains(&LONGITUDE_SIGNAL.to_string()),
        "Should receive longitude telemetry, got: {:?}",
        received_signals
    );
    assert!(
        received_signals.contains(&PARKING_SIGNAL.to_string()),
        "Should receive parking telemetry, got: {:?}",
        received_signals
    );

    telem_handle.abort();
}

/// TS-04-P5: Full Command Round-Trip.
///
/// Publish a lock command on NATS -> verify DATA_BROKER write -> write
/// response on DATA_BROKER -> verify response on NATS.
#[tokio::test]
async fn test_full_command_round_trip() {
    let vin = "TEST_VIN_TS04_P5";
    let nats_client = connect_nats(vin).await;
    let mut databroker = connect_databroker().await;

    // 1. Set up command processor
    let commands_sub = nats_client.subscribe_commands().await.expect("subscribe");
    let db_cmd = databroker.clone();
    let cmd_handle = tokio::spawn(async move {
        cloud_gateway_client::command_processor::run(
            commands_sub,
            db_cmd,
            "unused".to_string(),
        )
        .await;
    });

    // 2. Set up response relay
    let relay_nats = connect_nats(vin).await;
    let relay_db = connect_databroker().await;
    let relay_handle = tokio::spawn(async move {
        cloud_gateway_client::response_relay::run(relay_db, relay_nats, "unused".to_string())
            .await;
    });

    // Subscribe to command_responses on NATS
    let raw_nats = connect_raw_nats().await;
    let resp_subject = format!("vehicles.{}.command_responses", vin);
    let mut resp_sub = raw_nats.subscribe(resp_subject).await.expect("subscribe");

    // Give tasks a moment to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 3. Publish a valid lock command
    let cmd_subject = format!("vehicles.{}.commands", vin);
    let cmd_json = valid_command_json("aabbccdd-1234-5678-9abc-def012345678", "lock", vin);
    raw_nats
        .publish(cmd_subject, Bytes::from(cmd_json))
        .await
        .expect("publish");
    raw_nats.flush().await.expect("flush");

    // 4. Verify the command was written to DATA_BROKER
    tokio::time::sleep(Duration::from_secs(1)).await;
    let value = databroker
        .get_signal(DOOR_LOCK_SIGNAL)
        .await
        .expect("get_signal");
    match value {
        Some(SignalValue::String(s)) => {
            let parsed: serde_json::Value = serde_json::from_str(&s).expect("parse");
            assert_eq!(parsed["command_id"], "aabbccdd-1234-5678-9abc-def012345678");
        }
        other => panic!("Expected String signal, got {:?}", other),
    }

    // 5. Write a response to DATA_BROKER (simulating LOCKING_SERVICE)
    let response_json = serde_json::json!({
        "command_id": "aabbccdd-1234-5678-9abc-def012345678",
        "status": "success",
        "timestamp": 1700000001u64
    })
    .to_string();
    databroker
        .set_signal(
            DOOR_RESPONSE_SIGNAL,
            SignalValue::String(response_json),
        )
        .await
        .expect("set response");

    // 6. Verify the response is relayed to NATS
    let msg = tokio::time::timeout(Duration::from_secs(5), resp_sub.next())
        .await
        .expect("timeout waiting for response relay")
        .expect("subscription ended");

    let received: serde_json::Value =
        serde_json::from_slice(&msg.payload).expect("parse response");
    assert_eq!(
        received["command_id"],
        "aabbccdd-1234-5678-9abc-def012345678"
    );
    assert_eq!(received["status"], "success");

    cmd_handle.abort();
    relay_handle.abort();
}

/// TS-04-E1: Malformed Command JSON.
///
/// Verify that malformed JSON on the command subject is handled gracefully
/// and the service continues processing valid commands.
#[tokio::test]
async fn test_malformed_command_json() {
    let vin = "TEST_VIN_TS04_E1";
    let nats_client = connect_nats(vin).await;
    let mut databroker = connect_databroker().await;

    // Clear any previous value
    let _ = databroker
        .set_signal(
            DOOR_LOCK_SIGNAL,
            SignalValue::String("".to_string()),
        )
        .await;

    let commands_sub = nats_client.subscribe_commands().await.expect("subscribe");
    let db_clone = databroker.clone();
    let cmd_handle = tokio::spawn(async move {
        cloud_gateway_client::command_processor::run(
            commands_sub,
            db_clone,
            "unused".to_string(),
        )
        .await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let raw_nats = connect_raw_nats().await;
    let subject = format!("vehicles.{}.commands", vin);

    // Send malformed JSON
    raw_nats
        .publish(subject.clone(), Bytes::from("not valid json {{{"))
        .await
        .expect("publish");
    raw_nats.flush().await.expect("flush");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Now send a valid command
    let cmd_json = valid_command_json("ee0e8400-e29b-41d4-a716-446655440099", "lock", vin);
    raw_nats
        .publish(subject, Bytes::from(cmd_json))
        .await
        .expect("publish");
    raw_nats.flush().await.expect("flush");

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify the valid command was written
    let value = databroker
        .get_signal(DOOR_LOCK_SIGNAL)
        .await
        .expect("get_signal");
    match value {
        Some(SignalValue::String(s)) => {
            let parsed: serde_json::Value = serde_json::from_str(&s).expect("parse");
            assert_eq!(
                parsed["command_id"],
                "ee0e8400-e29b-41d4-a716-446655440099",
                "Valid command should be written after malformed one was discarded"
            );
        }
        other => panic!("Expected String signal, got {:?}", other),
    }

    cmd_handle.abort();
}

/// TS-04-E2: Command with Missing Required Fields.
///
/// Verify that a command JSON missing required fields is rejected and not
/// written to DATA_BROKER.
#[tokio::test]
async fn test_command_missing_required_fields() {
    let vin = "TEST_VIN_TS04_E2";
    let nats_client = connect_nats(vin).await;
    let mut databroker = connect_databroker().await;

    // Set a known value so we can verify it doesn't change
    let marker = "MARKER_E2_UNCHANGED";
    databroker
        .set_signal(DOOR_LOCK_SIGNAL, SignalValue::String(marker.to_string()))
        .await
        .expect("set marker");

    let commands_sub = nats_client.subscribe_commands().await.expect("subscribe");
    let db_clone = databroker.clone();
    let cmd_handle = tokio::spawn(async move {
        cloud_gateway_client::command_processor::run(
            commands_sub,
            db_clone,
            "unused".to_string(),
        )
        .await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let raw_nats = connect_raw_nats().await;
    let subject = format!("vehicles.{}.commands", vin);

    // Send command missing the `action` field
    let incomplete_json = serde_json::json!({
        "command_id": "550e8400-e29b-41d4-a716-446655440000",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": vin,
        "timestamp": 1700000000u64
    })
    .to_string();

    raw_nats
        .publish(subject, Bytes::from(incomplete_json))
        .await
        .expect("publish");
    raw_nats.flush().await.expect("flush");

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify the marker value is still there (command was rejected)
    let value = databroker
        .get_signal(DOOR_LOCK_SIGNAL)
        .await
        .expect("get_signal");
    match value {
        Some(SignalValue::String(s)) => {
            assert_eq!(
                s, marker,
                "Signal should still be the marker -- incomplete command should have been rejected"
            );
        }
        other => panic!("Expected marker String signal, got {:?}", other),
    }

    cmd_handle.abort();
}

/// TS-04-E3: Command with Invalid Action Value.
///
/// Verify that a command with an action other than "lock" or "unlock" is rejected.
#[tokio::test]
async fn test_command_invalid_action() {
    let vin = "TEST_VIN_TS04_E3";
    let nats_client = connect_nats(vin).await;
    let mut databroker = connect_databroker().await;

    // Set a known marker value
    let marker = "MARKER_E3_UNCHANGED";
    databroker
        .set_signal(DOOR_LOCK_SIGNAL, SignalValue::String(marker.to_string()))
        .await
        .expect("set marker");

    let commands_sub = nats_client.subscribe_commands().await.expect("subscribe");
    let db_clone = databroker.clone();
    let cmd_handle = tokio::spawn(async move {
        cloud_gateway_client::command_processor::run(
            commands_sub,
            db_clone,
            "unused".to_string(),
        )
        .await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let raw_nats = connect_raw_nats().await;
    let subject = format!("vehicles.{}.commands", vin);

    // Send command with invalid action "reboot"
    let invalid_cmd = valid_command_json("aa0e8400-e29b-41d4-a716-446655440005", "reboot", vin);
    raw_nats
        .publish(subject, Bytes::from(invalid_cmd))
        .await
        .expect("publish");
    raw_nats.flush().await.expect("flush");

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify the marker value is unchanged (invalid command was rejected)
    let value = databroker
        .get_signal(DOOR_LOCK_SIGNAL)
        .await
        .expect("get_signal");
    match value {
        Some(SignalValue::String(s)) => {
            assert_eq!(
                s, marker,
                "Signal should still be the marker -- invalid action should have been rejected"
            );
        }
        other => panic!("Expected marker String signal, got {:?}", other),
    }

    cmd_handle.abort();
}

/// TS-04-E5: VIN Isolation in NATS Subjects.
///
/// Verify that commands for other VINs are not processed by this client.
#[tokio::test]
async fn test_vin_isolation() {
    let vin_a = "VIN_AAA";
    let vin_b = "VIN_BBB";
    let nats_client = connect_nats(vin_a).await;
    let mut databroker = connect_databroker().await;

    // Set a known marker
    let marker = "MARKER_VIN_ISOLATION";
    databroker
        .set_signal(DOOR_LOCK_SIGNAL, SignalValue::String(marker.to_string()))
        .await
        .expect("set marker");

    let commands_sub = nats_client.subscribe_commands().await.expect("subscribe");
    let db_clone = databroker.clone();
    let cmd_handle = tokio::spawn(async move {
        cloud_gateway_client::command_processor::run(
            commands_sub,
            db_clone,
            "unused".to_string(),
        )
        .await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let raw_nats = connect_raw_nats().await;

    // Publish command for VIN_BBB (should NOT be processed by VIN_AAA's client)
    let other_vin_subject = format!("vehicles.{}.commands", vin_b);
    let other_cmd = valid_command_json("ff0e8400-e29b-41d4-a716-446655440010", "lock", vin_b);
    raw_nats
        .publish(other_vin_subject, Bytes::from(other_cmd))
        .await
        .expect("publish for other VIN");
    raw_nats.flush().await.expect("flush");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify marker is still there (other VIN's command was NOT processed)
    let value = databroker
        .get_signal(DOOR_LOCK_SIGNAL)
        .await
        .expect("get_signal");
    match &value {
        Some(SignalValue::String(s)) => {
            assert_eq!(
                s, marker,
                "Command for other VIN should not be processed"
            );
        }
        other => panic!("Expected marker, got {:?}", other),
    }

    // Now publish command for VIN_AAA (should be processed)
    let correct_subject = format!("vehicles.{}.commands", vin_a);
    let correct_cmd =
        valid_command_json("ff0e8400-e29b-41d4-a716-446655440011", "lock", vin_a);
    raw_nats
        .publish(correct_subject, Bytes::from(correct_cmd))
        .await
        .expect("publish for correct VIN");
    raw_nats.flush().await.expect("flush");

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify VIN_AAA's command was processed
    let value = databroker
        .get_signal(DOOR_LOCK_SIGNAL)
        .await
        .expect("get_signal");
    match value {
        Some(SignalValue::String(s)) => {
            let parsed: serde_json::Value = serde_json::from_str(&s).expect("parse");
            assert_eq!(
                parsed["command_id"],
                "ff0e8400-e29b-41d4-a716-446655440011",
                "Command for correct VIN should be processed"
            );
        }
        other => panic!("Expected String signal, got {:?}", other),
    }

    cmd_handle.abort();
}
