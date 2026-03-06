//! Integration tests for LOCKING_SERVICE.
//!
//! These tests require a running Kuksa Databroker instance with a UDS at
//! `/tmp/kuksa/databroker.sock` (or the path set in `DATABROKER_UDS_PATH`).
//!
//! Run with: `cargo test -p locking-service --features integration`
//!
//! Gated behind the `integration` feature flag so they are skipped
//! during normal `cargo test` runs.

#![cfg(feature = "integration")]

use locking_service::command::{Command, CommandResponse, ValidationError};
use locking_service::databroker_client::{DatabrokerClient, SignalValue};
use locking_service::safety;

/// VSS signal paths used in tests.
const SIGNAL_SPEED: &str = "Vehicle.Speed";
const SIGNAL_DOOR_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";
const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Get the DATA_BROKER address for integration tests.
///
/// Uses `DATABROKER_ADDR` env var if set (e.g. "http://localhost:55556"),
/// otherwise falls back to UDS at `DATABROKER_UDS_PATH` or the default path.
fn databroker_addr() -> (String, bool) {
    if let Ok(addr) = std::env::var("DATABROKER_ADDR") {
        (addr, true) // TCP
    } else {
        let path = std::env::var("DATABROKER_UDS_PATH")
            .unwrap_or_else(|_| "/tmp/kuksa/databroker.sock".to_string());
        (path, false) // UDS
    }
}

/// Connect to DATA_BROKER and return a client.
///
/// Tries TCP first (via `DATABROKER_ADDR`), falls back to UDS.
async fn connect() -> DatabrokerClient {
    let (addr, is_tcp) = databroker_addr();
    if is_tcp {
        DatabrokerClient::connect_tcp(&addr)
            .await
            .expect("Failed to connect to DATA_BROKER via TCP")
    } else {
        DatabrokerClient::connect(&addr)
            .await
            .expect("Failed to connect to DATA_BROKER via UDS")
    }
}

/// Set precondition signals: speed and door open state.
async fn set_preconditions(client: &mut DatabrokerClient, speed: f32, door_open: bool) {
    client
        .set_signal_float(SIGNAL_SPEED, speed)
        .await
        .expect("Failed to set Vehicle.Speed");
    client
        .set_signal_bool(SIGNAL_DOOR_OPEN, door_open)
        .await
        .expect("Failed to set door IsOpen");
}

/// Process a command JSON string through the full pipeline against the real DATA_BROKER.
///
/// This replicates the processing pipeline from `main.rs` using the public module APIs.
async fn process_command(client: &mut DatabrokerClient, cmd_json: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Step 1 & 2: Parse and validate the command
    let cmd = match Command::from_json(cmd_json) {
        Ok(cmd) => cmd,
        Err(e) => {
            let (reason, command_id) = match &e {
                ValidationError::MalformedJson(_) => {
                    ("invalid_command".to_string(), "unknown".to_string())
                }
                ValidationError::MissingField(_) => {
                    let cid = extract_command_id(cmd_json).unwrap_or_else(|| "unknown".to_string());
                    ("invalid_command".to_string(), cid)
                }
                ValidationError::InvalidAction(_) => {
                    let cid = extract_command_id(cmd_json).unwrap_or_else(|| "unknown".to_string());
                    ("invalid_action".to_string(), cid)
                }
            };

            let response = CommandResponse::failure(command_id, reason, now);
            write_response(client, &response).await;
            return;
        }
    };

    let command_id = cmd.command_id.clone();

    // Step 3: Safety constraint checks
    let speed = read_speed(client).await;
    let door_open = read_door_open(client).await;

    if let Err(reason) = safety::check_safety_constraints(speed, door_open) {
        let response = CommandResponse::failure(command_id, reason, now);
        write_response(client, &response).await;
        return;
    }

    // Step 4: Execute lock/unlock
    if let Err(_e) = locking_service::executor::execute_lock_action(client, &cmd.action).await {
        let response =
            CommandResponse::failure(command_id, "execution_failed".to_string(), now);
        write_response(client, &response).await;
        return;
    }

    // Step 5: Write success response
    let response = CommandResponse::success(command_id, now);
    write_response(client, &response).await;
}

/// Read Vehicle.Speed from DATA_BROKER.
async fn read_speed(client: &mut DatabrokerClient) -> Option<f64> {
    match client.get_signal(SIGNAL_SPEED).await {
        Ok(Some(SignalValue::Float(f))) => Some(f as f64),
        Ok(Some(SignalValue::Double(d))) => Some(d),
        Ok(Some(SignalValue::Int32(i))) => Some(i as f64),
        Ok(Some(SignalValue::Uint32(u))) => Some(u as f64),
        _ => None,
    }
}

/// Read door IsOpen from DATA_BROKER.
async fn read_door_open(client: &mut DatabrokerClient) -> Option<bool> {
    match client.get_signal(SIGNAL_DOOR_OPEN).await {
        Ok(Some(SignalValue::Bool(b))) => Some(b),
        _ => None,
    }
}

/// Read IsLocked from DATA_BROKER.
async fn read_is_locked(client: &mut DatabrokerClient) -> Option<bool> {
    match client.get_signal(SIGNAL_IS_LOCKED).await {
        Ok(Some(SignalValue::Bool(b))) => Some(b),
        _ => None,
    }
}

/// Read the latest response JSON from DATA_BROKER.
async fn read_response(client: &mut DatabrokerClient) -> Option<serde_json::Value> {
    match client.get_signal(SIGNAL_RESPONSE).await {
        Ok(Some(SignalValue::String(s))) => serde_json::from_str(&s).ok(),
        _ => None,
    }
}

/// Write a command response to DATA_BROKER.
async fn write_response(client: &mut DatabrokerClient, response: &CommandResponse) {
    let json = response.to_json();
    client
        .set_signal_string(SIGNAL_RESPONSE, &json)
        .await
        .expect("Failed to write response");
}

/// Extract command_id from raw JSON (best-effort).
fn extract_command_id(json_str: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(json_str)
        .ok()?
        .get("command_id")?
        .as_str()
        .map(|s| s.to_string())
}

// ---------------------------------------------------------------------------
// Integration Tests
// ---------------------------------------------------------------------------

/// TS-03-1: Lock command happy path.
///
/// Preconditions: Vehicle.Speed = 0.0, door closed.
/// Write a valid lock command -> verify IsLocked = true and success response.
#[tokio::test]
async fn test_lock_command_happy_path() {
    let mut client = connect().await;

    // Set preconditions: stationary, door closed
    set_preconditions(&mut client, 0.0, false).await;

    let cmd_json = r#"{
        "command_id": "550e8400-e29b-41d4-a716-446655440000",
        "action": "lock",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000000
    }"#;

    process_command(&mut client, cmd_json).await;

    // Verify IsLocked is true
    let is_locked = read_is_locked(&mut client).await;
    assert_eq!(is_locked, Some(true), "Door should be locked after lock command");

    // Verify success response
    let response = read_response(&mut client).await.expect("Response should be written");
    assert_eq!(response["command_id"], "550e8400-e29b-41d4-a716-446655440000");
    assert_eq!(response["status"], "success");
    assert!(response.get("timestamp").is_some(), "Response should have a timestamp");
}

/// TS-03-2: Unlock command happy path.
///
/// Preconditions: Vehicle.Speed = 0.0, door closed, IsLocked = true.
/// Write unlock command -> verify IsLocked = false and success response.
#[tokio::test]
async fn test_unlock_command_happy_path() {
    let mut client = connect().await;

    // Set preconditions: stationary, door closed, currently locked
    set_preconditions(&mut client, 0.0, false).await;
    client
        .set_signal_bool(SIGNAL_IS_LOCKED, true)
        .await
        .expect("Failed to set IsLocked");

    let cmd_json = r#"{
        "command_id": "660e8400-e29b-41d4-a716-446655440001",
        "action": "unlock",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000010
    }"#;

    process_command(&mut client, cmd_json).await;

    // Verify IsLocked is false
    let is_locked = read_is_locked(&mut client).await;
    assert_eq!(is_locked, Some(false), "Door should be unlocked after unlock command");

    // Verify success response
    let response = read_response(&mut client).await.expect("Response should be written");
    assert_eq!(response["command_id"], "660e8400-e29b-41d4-a716-446655440001");
    assert_eq!(response["status"], "success");
    assert!(response.get("timestamp").is_some());
}

/// TS-03-3: Safety constraint rejection -- vehicle moving.
///
/// Preconditions: Vehicle.Speed = 30.0, door closed.
/// Write lock command -> verify rejection with reason "vehicle_moving".
#[tokio::test]
async fn test_safety_rejection_vehicle_moving() {
    let mut client = connect().await;

    // Set preconditions: moving, door closed
    set_preconditions(&mut client, 30.0, false).await;

    // Record current lock state to verify it doesn't change
    let lock_before = read_is_locked(&mut client).await;

    let cmd_json = r#"{
        "command_id": "770e8400-e29b-41d4-a716-446655440002",
        "action": "lock",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000020
    }"#;

    process_command(&mut client, cmd_json).await;

    // Verify lock state unchanged
    let lock_after = read_is_locked(&mut client).await;
    assert_eq!(lock_before, lock_after, "Lock state should not change when vehicle is moving");

    // Verify failure response
    let response = read_response(&mut client).await.expect("Response should be written");
    assert_eq!(response["command_id"], "770e8400-e29b-41d4-a716-446655440002");
    assert_eq!(response["status"], "failed");
    assert_eq!(response["reason"], "vehicle_moving");
}

/// TS-03-4: Safety constraint rejection -- door ajar.
///
/// Preconditions: Vehicle.Speed = 0.0, door open.
/// Write lock command -> verify rejection with reason "door_ajar".
#[tokio::test]
async fn test_safety_rejection_door_ajar() {
    let mut client = connect().await;

    // Set preconditions: stationary, door open
    set_preconditions(&mut client, 0.0, true).await;

    // Record current lock state
    let lock_before = read_is_locked(&mut client).await;

    let cmd_json = r#"{
        "command_id": "880e8400-e29b-41d4-a716-446655440003",
        "action": "lock",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000030
    }"#;

    process_command(&mut client, cmd_json).await;

    // Verify lock state unchanged
    let lock_after = read_is_locked(&mut client).await;
    assert_eq!(lock_before, lock_after, "Lock state should not change when door is ajar");

    // Verify failure response
    let response = read_response(&mut client).await.expect("Response should be written");
    assert_eq!(response["command_id"], "880e8400-e29b-41d4-a716-446655440003");
    assert_eq!(response["status"], "failed");
    assert_eq!(response["reason"], "door_ajar");
}

/// TS-03-E1: Invalid command JSON handling.
///
/// Write malformed JSON -> verify failure response with reason "invalid_command".
/// Then write valid command -> verify service still processes it.
#[tokio::test]
async fn test_invalid_json_handling() {
    let mut client = connect().await;
    set_preconditions(&mut client, 0.0, false).await;

    // Step 1: Send malformed JSON
    process_command(&mut client, "not valid json {{{").await;

    // Verify failure response
    let response = read_response(&mut client).await.expect("Response should be written");
    assert_eq!(response["status"], "failed");
    assert_eq!(response["reason"], "invalid_command");

    // Step 2: Send valid command to verify continued processing
    let cmd_json = r#"{
        "command_id": "bb0e8400-e29b-41d4-a716-446655440006",
        "action": "lock",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000060
    }"#;

    process_command(&mut client, cmd_json).await;

    let response = read_response(&mut client).await.expect("Response should be written");
    assert_eq!(response["command_id"], "bb0e8400-e29b-41d4-a716-446655440006");
    assert_eq!(response["status"], "success");
}

/// TS-03-E2: Command with missing required fields.
///
/// Write JSON missing 'action' -> verify failure response with reason "invalid_command".
#[tokio::test]
async fn test_missing_fields_handling() {
    let mut client = connect().await;
    set_preconditions(&mut client, 0.0, false).await;

    // Record current lock state
    let lock_before = read_is_locked(&mut client).await;

    let cmd_json = r#"{
        "command_id": "990e8400-e29b-41d4-a716-446655440004",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000040
    }"#;

    process_command(&mut client, cmd_json).await;

    // Verify lock state unchanged
    let lock_after = read_is_locked(&mut client).await;
    assert_eq!(lock_before, lock_after, "Lock state should not change on invalid command");

    // Verify failure response
    let response = read_response(&mut client).await.expect("Response should be written");
    assert_eq!(response["status"], "failed");
    assert_eq!(response["reason"], "invalid_command");
}

/// TS-03-E3: Command with invalid action value.
///
/// Write command with action="reboot" -> verify failure with reason "invalid_action".
#[tokio::test]
async fn test_invalid_action_handling() {
    let mut client = connect().await;
    set_preconditions(&mut client, 0.0, false).await;

    // Record current lock state
    let lock_before = read_is_locked(&mut client).await;

    let cmd_json = r#"{
        "command_id": "aa0e8400-e29b-41d4-a716-446655440005",
        "action": "reboot",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000050
    }"#;

    process_command(&mut client, cmd_json).await;

    // Verify lock state unchanged
    let lock_after = read_is_locked(&mut client).await;
    assert_eq!(lock_before, lock_after, "Lock state should not change on invalid action");

    // Verify failure response
    let response = read_response(&mut client).await.expect("Response should be written");
    assert_eq!(response["command_id"], "aa0e8400-e29b-41d4-a716-446655440005");
    assert_eq!(response["status"], "failed");
    assert_eq!(response["reason"], "invalid_action");
}

/// TS-03-E4: Command response format validation.
///
/// Verify that success and failure responses conform to expected JSON format.
#[tokio::test]
async fn test_response_format_validation() {
    let mut client = connect().await;

    // Test 1: Trigger a successful lock command
    set_preconditions(&mut client, 0.0, false).await;

    let cmd_json = r#"{
        "command_id": "cc0e8400-e29b-41d4-a716-446655440007",
        "action": "lock",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000070
    }"#;

    process_command(&mut client, cmd_json).await;

    let success_resp = read_response(&mut client).await.expect("Success response should exist");

    // Validate success response format
    assert!(success_resp["command_id"].is_string(), "command_id should be a string");
    assert_eq!(success_resp["status"], "success");
    assert!(success_resp.get("reason").is_none(), "Success response should not have reason");
    assert!(success_resp["timestamp"].is_u64(), "timestamp should be an integer");
    let ts = success_resp["timestamp"].as_u64().unwrap();
    assert!(ts > 0, "timestamp should be a valid positive Unix timestamp");

    // Test 2: Trigger a failed command (vehicle moving)
    set_preconditions(&mut client, 50.0, false).await;

    let cmd_json = r#"{
        "command_id": "dd0e8400-e29b-41d4-a716-446655440008",
        "action": "lock",
        "doors": ["driver"],
        "source": "companion_app",
        "vin": "TEST_VIN_001",
        "timestamp": 1700000080
    }"#;

    process_command(&mut client, cmd_json).await;

    let failure_resp = read_response(&mut client).await.expect("Failure response should exist");

    // Validate failure response format
    assert!(failure_resp["command_id"].is_string(), "command_id should be a string");
    assert_eq!(failure_resp["status"], "failed");
    assert!(failure_resp["reason"].is_string(), "Failure response should have a reason string");
    assert!(failure_resp["timestamp"].is_u64(), "timestamp should be an integer");
    let ts = failure_resp["timestamp"].as_u64().unwrap();
    assert!(ts > 0, "timestamp should be a valid positive Unix timestamp");
}

/// TS-03-P1: Safety invariant -- no state change on constraint violation.
///
/// Property: for various speed/door combinations where constraints are violated,
/// IsLocked must remain unchanged.
#[tokio::test]
async fn test_safety_invariant_no_state_change() {
    let mut client = connect().await;

    // Set initial locked state
    client
        .set_signal_bool(SIGNAL_IS_LOCKED, false)
        .await
        .expect("Failed to set initial lock state");

    // Test cases: (speed, door_open) where at least one constraint is violated
    let unsafe_conditions: Vec<(f32, bool)> = vec![
        (1.0, false),   // speed at threshold
        (5.0, false),   // speed above threshold
        (100.0, false), // high speed
        (0.0, true),    // door ajar
        (50.0, true),   // both violated
    ];

    for (speed, door_open) in unsafe_conditions {
        set_preconditions(&mut client, speed, door_open).await;

        let lock_before = read_is_locked(&mut client).await;

        let cmd_json = format!(
            r#"{{
                "command_id": "prop-test-{}-{}",
                "action": "lock",
                "doors": ["driver"],
                "source": "test",
                "vin": "TEST",
                "timestamp": 1700000000
            }}"#,
            speed, door_open
        );

        process_command(&mut client, &cmd_json).await;

        let lock_after = read_is_locked(&mut client).await;
        assert_eq!(
            lock_before, lock_after,
            "Lock state must not change when speed={} door_open={}",
            speed, door_open
        );
    }

    // Also test unlock action with unsafe conditions
    client
        .set_signal_bool(SIGNAL_IS_LOCKED, true)
        .await
        .expect("Failed to set lock state");

    for (speed, door_open) in [(10.0, false), (0.0, true)] {
        set_preconditions(&mut client, speed, door_open).await;

        let lock_before = read_is_locked(&mut client).await;

        let cmd_json = format!(
            r#"{{
                "command_id": "prop-unlock-{}-{}",
                "action": "unlock",
                "doors": ["driver"],
                "source": "test",
                "vin": "TEST",
                "timestamp": 1700000000
            }}"#,
            speed, door_open
        );

        process_command(&mut client, &cmd_json).await;

        let lock_after = read_is_locked(&mut client).await;
        assert_eq!(
            lock_before, lock_after,
            "Lock state must not change on unlock when speed={} door_open={}",
            speed, door_open
        );
    }
}

/// TS-03-P2: Response completeness -- every command gets a response.
///
/// Property: for all command inputs (valid, invalid, malformed), processing
/// produces a response with status "success" or "failed".
#[tokio::test]
async fn test_response_completeness() {
    let mut client = connect().await;
    set_preconditions(&mut client, 0.0, false).await;

    let test_commands = vec![
        // Valid lock
        r#"{"command_id":"p2-1","action":"lock","doors":["driver"],"source":"t","vin":"V","timestamp":1}"#,
        // Valid unlock
        r#"{"command_id":"p2-2","action":"unlock","doors":["driver"],"source":"t","vin":"V","timestamp":2}"#,
        // Invalid action
        r#"{"command_id":"p2-3","action":"reboot","doors":["driver"],"source":"t","vin":"V","timestamp":3}"#,
        // Missing fields
        r#"{"command_id":"p2-4","doors":["driver"],"source":"t","vin":"V","timestamp":4}"#,
        // Malformed JSON
        "not json at all",
    ];

    for (i, cmd_json) in test_commands.iter().enumerate() {
        // Clear response before each test
        client
            .set_signal_string(SIGNAL_RESPONSE, "")
            .await
            .expect("Failed to clear response");

        process_command(&mut client, cmd_json).await;

        let response = read_response(&mut client).await;
        assert!(
            response.is_some(),
            "Command {} should produce a response: {}",
            i, cmd_json
        );

        let resp = response.unwrap();
        let status = resp["status"].as_str().expect("Response should have status");
        assert!(
            status == "success" || status == "failed",
            "Command {} status should be 'success' or 'failed', got: {}",
            i, status
        );
    }
}

/// TS-03-P3: Safety invariant -- successful execution implies safe state.
///
/// Property: if response is "success", then speed was < 1.0 and door was closed
/// at the time of execution.
#[tokio::test]
async fn test_success_implies_safe_state() {
    let mut client = connect().await;

    // Test: set safe conditions, execute, verify success
    set_preconditions(&mut client, 0.0, false).await;

    let cmd_json = r#"{
        "command_id": "p3-safe",
        "action": "lock",
        "doors": ["driver"],
        "source": "test",
        "vin": "TEST",
        "timestamp": 1700000000
    }"#;

    process_command(&mut client, cmd_json).await;

    let response = read_response(&mut client).await.expect("Should have response");
    assert_eq!(response["status"], "success");

    // Verify the preconditions at the time were safe
    let speed = read_speed(&mut client).await;
    let door = read_door_open(&mut client).await;
    assert!(speed.unwrap_or(0.0) < 1.0, "Speed should be < 1.0 for success");
    assert!(!door.unwrap_or(false), "Door should be closed for success");

    // Test: set unsafe conditions, execute, verify NOT success
    set_preconditions(&mut client, 5.0, false).await;

    let cmd_json = r#"{
        "command_id": "p3-unsafe",
        "action": "lock",
        "doors": ["driver"],
        "source": "test",
        "vin": "TEST",
        "timestamp": 1700000001
    }"#;

    process_command(&mut client, cmd_json).await;

    let response = read_response(&mut client).await.expect("Should have response");
    assert_ne!(
        response["status"], "success",
        "Command should not succeed when vehicle is moving"
    );
}
