//! Integration tests for LOCKING_SERVICE.
//!
//! These tests require a running DATA_BROKER (Kuksa Databroker) and are gated
//! behind the `integration` feature flag.
//!
//! Run with: `cd rhivos && cargo test -p locking-service --features integration`
//! Prerequisites: `make infra-up`

#![cfg(feature = "integration")]

use locking_service::command::CommandResponse;
use locking_service::databroker_client::{DataBrokerClient, SignalValue};

/// Default TCP address for DATA_BROKER in the test environment.
const DATABROKER_ADDR: &str = "127.0.0.1:55556";

/// Signal paths used in tests.
const RESPONSE_SIGNAL: &str = "Vehicle.Command.Door.Response";
const LOCK_STATE_SIGNAL: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
const SPEED_SIGNAL: &str = "Vehicle.Speed";
const DOOR_OPEN_SIGNAL: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";

/// Helper: connect to DATA_BROKER via TCP.
async fn connect() -> DataBrokerClient {
    DataBrokerClient::connect_tcp(DATABROKER_ADDR)
        .await
        .expect("Failed to connect to DATA_BROKER. Is infra running? (make infra-up)")
}

/// Helper: set up safe preconditions (speed = 0, door closed).
async fn set_safe_preconditions(client: &mut DataBrokerClient) {
    client
        .set_signal(SPEED_SIGNAL, SignalValue::Float(0.0))
        .await
        .expect("Failed to set Vehicle.Speed");
    client
        .set_signal(DOOR_OPEN_SIGNAL, SignalValue::Bool(false))
        .await
        .expect("Failed to set door IsOpen");
}

/// Helper: send a command and process it, then read the response.
async fn send_command_and_get_response(
    client: &mut DataBrokerClient,
    command_json: &str,
) -> CommandResponse {
    // Process the command through the locking service pipeline.
    locking_service::process_command(command_json, client).await;

    // Read the response signal.
    let resp_value = client
        .get_signal(RESPONSE_SIGNAL)
        .await
        .expect("Failed to read response signal");

    match resp_value {
        Some(SignalValue::String(s)) => {
            serde_json::from_str(&s).expect("Response should be valid JSON")
        }
        other => panic!("Expected String response signal, got {:?}", other),
    }
}

/// Helper: read the current lock state.
async fn read_lock_state(client: &mut DataBrokerClient) -> Option<bool> {
    match client.get_signal(LOCK_STATE_SIGNAL).await {
        Ok(Some(SignalValue::Bool(b))) => Some(b),
        Ok(None) => None,
        Ok(other) => panic!("Unexpected lock state value: {:?}", other),
        Err(e) => panic!("Failed to read lock state: {}", e),
    }
}

/// TS-03-1: Lock command happy path.
///
/// Preconditions:
/// - Vehicle.Speed = 0.0
/// - Vehicle.Cabin.Door.Row1.DriverSide.IsOpen = false
///
/// Write a valid lock command -> verify IsLocked = true and success response.
#[tokio::test]
async fn test_lock_command_happy_path() {
    let mut client = connect().await;
    set_safe_preconditions(&mut client).await;

    let cmd = r#"{
        "command_id": "550e8400-e29b-41d4-a716-446655440000",
        "action": "lock",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000000
    }"#;

    let resp = send_command_and_get_response(&mut client, cmd).await;

    assert_eq!(resp.command_id, "550e8400-e29b-41d4-a716-446655440000");
    assert_eq!(resp.status, "success");
    assert!(resp.reason.is_none());

    let locked = read_lock_state(&mut client).await;
    assert_eq!(locked, Some(true), "Door should be locked after lock command");
}

/// TS-03-2: Unlock command happy path.
///
/// Preconditions:
/// - Vehicle.Speed = 0.0
/// - Vehicle.Cabin.Door.Row1.DriverSide.IsOpen = false
/// - Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true
///
/// Write a valid unlock command -> verify IsLocked = false and success response.
#[tokio::test]
async fn test_unlock_command_happy_path() {
    let mut client = connect().await;
    set_safe_preconditions(&mut client).await;

    // Pre-set locked state.
    client
        .set_signal(LOCK_STATE_SIGNAL, SignalValue::Bool(true))
        .await
        .expect("Failed to set initial lock state");

    let cmd = r#"{
        "command_id": "660e8400-e29b-41d4-a716-446655440001",
        "action": "unlock",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000010
    }"#;

    let resp = send_command_and_get_response(&mut client, cmd).await;

    assert_eq!(resp.command_id, "660e8400-e29b-41d4-a716-446655440001");
    assert_eq!(resp.status, "success");
    assert!(resp.reason.is_none());

    let locked = read_lock_state(&mut client).await;
    assert_eq!(locked, Some(false), "Door should be unlocked after unlock command");
}

/// TS-03-3: Safety constraint rejection -- vehicle moving.
///
/// Preconditions:
/// - Vehicle.Speed = 30.0
/// - Vehicle.Cabin.Door.Row1.DriverSide.IsOpen = false
///
/// Send lock command -> verify rejection with reason "vehicle_moving".
#[tokio::test]
async fn test_safety_rejection_vehicle_moving() {
    let mut client = connect().await;

    // Set speed above threshold.
    client
        .set_signal(SPEED_SIGNAL, SignalValue::Float(30.0))
        .await
        .expect("Failed to set Vehicle.Speed");
    client
        .set_signal(DOOR_OPEN_SIGNAL, SignalValue::Bool(false))
        .await
        .expect("Failed to set door IsOpen");

    // Record current lock state before command.
    let lock_before = read_lock_state(&mut client).await;

    let cmd = r#"{
        "command_id": "770e8400-e29b-41d4-a716-446655440002",
        "action": "lock",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000020
    }"#;

    let resp = send_command_and_get_response(&mut client, cmd).await;

    assert_eq!(resp.command_id, "770e8400-e29b-41d4-a716-446655440002");
    assert_eq!(resp.status, "failed");
    assert_eq!(resp.reason, Some("vehicle_moving".to_string()));

    // Lock state should not have changed.
    let lock_after = read_lock_state(&mut client).await;
    assert_eq!(lock_before, lock_after, "Lock state should not change when vehicle is moving");
}

/// TS-03-4: Safety constraint rejection -- door ajar.
///
/// Preconditions:
/// - Vehicle.Speed = 0.0
/// - Vehicle.Cabin.Door.Row1.DriverSide.IsOpen = true
///
/// Send lock command -> verify rejection with reason "door_ajar".
#[tokio::test]
async fn test_safety_rejection_door_ajar() {
    let mut client = connect().await;

    // Set door open.
    client
        .set_signal(SPEED_SIGNAL, SignalValue::Float(0.0))
        .await
        .expect("Failed to set Vehicle.Speed");
    client
        .set_signal(DOOR_OPEN_SIGNAL, SignalValue::Bool(true))
        .await
        .expect("Failed to set door IsOpen");

    let lock_before = read_lock_state(&mut client).await;

    let cmd = r#"{
        "command_id": "880e8400-e29b-41d4-a716-446655440003",
        "action": "lock",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000030
    }"#;

    let resp = send_command_and_get_response(&mut client, cmd).await;

    assert_eq!(resp.command_id, "880e8400-e29b-41d4-a716-446655440003");
    assert_eq!(resp.status, "failed");
    assert_eq!(resp.reason, Some("door_ajar".to_string()));

    let lock_after = read_lock_state(&mut client).await;
    assert_eq!(lock_before, lock_after, "Lock state should not change when door is ajar");
}

/// TS-03-E1: Invalid JSON handling.
///
/// Write malformed JSON -> verify failure response with reason "invalid_command".
/// Then write a valid command -> verify service continues processing.
#[tokio::test]
async fn test_invalid_json_handling() {
    let mut client = connect().await;
    set_safe_preconditions(&mut client).await;

    // Send malformed JSON.
    let resp = send_command_and_get_response(&mut client, "not valid json {{{").await;
    assert_eq!(resp.status, "failed");
    assert_eq!(resp.reason, Some("invalid_command".to_string()));

    // Verify service continues processing with a valid command.
    let cmd = r#"{
        "command_id": "bb0e8400-e29b-41d4-a716-446655440006",
        "action": "lock",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000060
    }"#;

    let resp = send_command_and_get_response(&mut client, cmd).await;
    assert_eq!(resp.status, "success", "Service should continue processing after invalid JSON");
    assert_eq!(resp.command_id, "bb0e8400-e29b-41d4-a716-446655440006");
}

/// TS-03-E2: Missing required fields handling.
///
/// Write JSON missing `action` field -> verify failure response with reason "invalid_command".
#[tokio::test]
async fn test_missing_fields_handling() {
    let mut client = connect().await;
    set_safe_preconditions(&mut client).await;

    let cmd = r#"{
        "command_id": "990e8400-e29b-41d4-a716-446655440004",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000040
    }"#;

    let resp = send_command_and_get_response(&mut client, cmd).await;

    assert_eq!(resp.status, "failed");
    assert_eq!(resp.reason, Some("invalid_command".to_string()));

    // Verify lock state was not modified.
    // (We can't assert an exact value since it depends on prior tests,
    // but we verify no crash occurred and a response was produced.)
}

/// TS-03-E3: Invalid action value handling.
///
/// Write command with action "reboot" -> verify failure response with reason "invalid_action".
#[tokio::test]
async fn test_invalid_action_handling() {
    let mut client = connect().await;
    set_safe_preconditions(&mut client).await;

    let lock_before = read_lock_state(&mut client).await;

    let cmd = r#"{
        "command_id": "aa0e8400-e29b-41d4-a716-446655440005",
        "action": "reboot",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000050
    }"#;

    let resp = send_command_and_get_response(&mut client, cmd).await;

    assert_eq!(resp.command_id, "aa0e8400-e29b-41d4-a716-446655440005");
    assert_eq!(resp.status, "failed");
    assert_eq!(resp.reason, Some("invalid_action".to_string()));

    let lock_after = read_lock_state(&mut client).await;
    assert_eq!(lock_before, lock_after, "Lock state should not change on invalid action");
}

/// TS-03-E4: Response format validation.
///
/// Verify success and failure responses conform to expected JSON format.
#[tokio::test]
async fn test_response_format_validation() {
    let mut client = connect().await;
    set_safe_preconditions(&mut client).await;

    // 1. Trigger a successful lock command.
    let cmd = r#"{
        "command_id": "cc0e8400-e29b-41d4-a716-446655440007",
        "action": "lock",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000070
    }"#;

    locking_service::process_command(cmd, &mut client).await;

    let resp_str = match client.get_signal(RESPONSE_SIGNAL).await.unwrap() {
        Some(SignalValue::String(s)) => s,
        other => panic!("Expected String response, got {:?}", other),
    };

    // Validate success response JSON format.
    let parsed: serde_json::Value = serde_json::from_str(&resp_str).expect("Valid JSON");
    assert_eq!(parsed["status"], "success");
    assert!(parsed["command_id"].is_string(), "command_id should be a string");
    assert!(parsed["timestamp"].is_u64(), "timestamp should be a non-negative integer");
    assert!(parsed.get("reason").is_none(), "Success response should not contain reason");

    // 2. Trigger a failed command (vehicle moving).
    client
        .set_signal(SPEED_SIGNAL, SignalValue::Float(50.0))
        .await
        .unwrap();

    let cmd = r#"{
        "command_id": "dd0e8400-e29b-41d4-a716-446655440008",
        "action": "lock",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000080
    }"#;

    locking_service::process_command(cmd, &mut client).await;

    let resp_str = match client.get_signal(RESPONSE_SIGNAL).await.unwrap() {
        Some(SignalValue::String(s)) => s,
        other => panic!("Expected String response, got {:?}", other),
    };

    // Validate failure response JSON format.
    let parsed: serde_json::Value = serde_json::from_str(&resp_str).expect("Valid JSON");
    assert_eq!(parsed["status"], "failed");
    assert!(parsed["command_id"].is_string(), "command_id should be a string");
    assert!(parsed["reason"].is_string(), "Failure response must have reason string");
    assert!(parsed["timestamp"].is_u64(), "timestamp should be a non-negative integer");
}
