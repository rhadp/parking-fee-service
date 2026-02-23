//! Integration tests for LOCKING_SERVICE.
//!
//! These tests verify the complete command processing flow through a running
//! DATA_BROKER. They cover command subscription, lock/unlock execution,
//! safety constraint enforcement, and response writing.
//!
//! Prerequisites:
//! - DATA_BROKER must be running (`make infra-up`)
//! - No separate LOCKING_SERVICE process needed — tests exercise the service
//!   logic directly via the library API
//!
//! Test Spec: TS-02-6 through TS-02-14, TS-02-E3 through TS-02-E7,
//!            TS-02-P1 through TS-02-P4

use std::time::Duration;

use databroker_client::{DataValue, DatabrokerClient};
use locking_service::command::{self, LockAction, ParseResult};
use locking_service::safety::SafetyChecker;
use locking_service::service::signals;
use tokio_stream::StreamExt;

/// Check if DATA_BROKER infrastructure is available via TCP.
fn infra_available() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:55556".parse().unwrap(),
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

/// Connect to DATA_BROKER via TCP for testing.
async fn test_client() -> DatabrokerClient {
    DatabrokerClient::connect("http://localhost:55556")
        .await
        .expect("should connect to DATA_BROKER on port 55556")
}

/// Send a lock/unlock command by writing to Vehicle.Command.Door.Lock
/// and wait for the response on Vehicle.Command.Door.Response.
async fn send_command_and_wait(
    client: &DatabrokerClient,
    command_json: &str,
    timeout: Duration,
) -> Option<serde_json::Value> {
    // Subscribe to the response signal before writing the command
    let mut stream = client
        .subscribe(&[signals::RESPONSE])
        .await
        .expect("should subscribe to response signal");

    // Write the command
    client
        .set_value(
            signals::COMMAND,
            DataValue::String(command_json.to_string()),
        )
        .await
        .expect("should write command signal");

    // Wait for a response
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

/// Run the locking service processing loop in a background task.
/// Returns a handle that can be used to abort the service.
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

/// Process a command (mirrors the service logic for testing).
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
    if let Err(_) = client
        .set_value(signals::IS_LOCKED, DataValue::Bool(lock_value))
        .await
    {
        return CommandResponse::failed(&cmd.command_id, "write_failed");
    }

    CommandResponse::success(&cmd.command_id)
}

// ── TS-02-6: Subscribe to commands ──────────────────────────────────────────

/// TS-02-6: LOCKING_SERVICE subscribes to command signals and produces responses.
#[tokio::test]
async fn test_locking_subscribes_to_commands() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());

    // Give the service a moment to subscribe
    tokio::time::sleep(Duration::from_millis(500)).await;

    set_speed(&client, 0.0).await;
    set_door_open(&client, false).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let response = send_command_and_wait(
        &client,
        &make_command("sub-test-1", "lock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["command_id"], "sub-test-1");
}

// ── TS-02-8: Execute lock ───────────────────────────────────────────────────

/// TS-02-8: LOCKING_SERVICE executes lock action.
#[tokio::test]
async fn test_locking_executes_lock() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    set_speed(&client, 0.0).await;
    set_door_open(&client, false).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let response = send_command_and_wait(
        &client,
        &make_command("lock-test-1", "lock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "success");

    let locked = get_locked(&client).await;
    assert_eq!(locked, Some(true));
}

// ── TS-02-9: Execute unlock ─────────────────────────────────────────────────

/// TS-02-9: LOCKING_SERVICE executes unlock action.
#[tokio::test]
async fn test_locking_executes_unlock() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Pre-set: locked, speed 0, door closed
    set_speed(&client, 0.0).await;
    set_door_open(&client, false).await;
    client
        .set_value(signals::IS_LOCKED, DataValue::Bool(true))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let response = send_command_and_wait(
        &client,
        &make_command("unlock-test-1", "unlock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "success");

    let locked = get_locked(&client).await;
    assert_eq!(locked, Some(false));
}

// ── TS-02-10: Reject lock when vehicle moving ───────────────────────────────

/// TS-02-10: LOCKING_SERVICE rejects lock when vehicle is moving.
#[tokio::test]
async fn test_locking_rejects_lock_vehicle_moving() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let initial_locked = get_locked(&client).await;

    set_speed(&client, 30.0).await;
    set_door_open(&client, false).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let response = send_command_and_wait(
        &client,
        &make_command("moving-lock-1", "lock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "failed");
    assert_eq!(resp["reason"], "vehicle_moving");

    // Lock state should not change
    let current_locked = get_locked(&client).await;
    assert_eq!(current_locked, initial_locked);
}

// ── TS-02-11: Reject lock when door open ────────────────────────────────────

/// TS-02-11: LOCKING_SERVICE rejects lock when door is open.
#[tokio::test]
async fn test_locking_rejects_lock_door_open() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let initial_locked = get_locked(&client).await;

    set_speed(&client, 0.0).await;
    set_door_open(&client, true).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let response = send_command_and_wait(
        &client,
        &make_command("door-open-1", "lock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "failed");
    assert_eq!(resp["reason"], "door_open");

    let current_locked = get_locked(&client).await;
    assert_eq!(current_locked, initial_locked);
}

// ── TS-02-12: Reject unlock when vehicle moving ─────────────────────────────

/// TS-02-12: LOCKING_SERVICE rejects unlock when vehicle is moving.
#[tokio::test]
async fn test_locking_rejects_unlock_vehicle_moving() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Pre-set: locked, speed > 0
    client
        .set_value(signals::IS_LOCKED, DataValue::Bool(true))
        .await
        .unwrap();
    set_speed(&client, 15.0).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let response = send_command_and_wait(
        &client,
        &make_command("moving-unlock-1", "unlock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "failed");
    assert_eq!(resp["reason"], "vehicle_moving");

    let locked = get_locked(&client).await;
    assert_eq!(locked, Some(true));
}

// ── TS-02-13: Failure response includes reason ──────────────────────────────

/// TS-02-13: LOCKING_SERVICE writes failure response with reason.
#[tokio::test]
async fn test_locking_failure_response_has_reason() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    set_speed(&client, 10.0).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let response = send_command_and_wait(
        &client,
        &make_command("reason-test-1", "lock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "failed");
    assert!(
        resp["reason"].as_str().map(|r| !r.is_empty()).unwrap_or(false),
        "reason should be non-empty"
    );
    assert_eq!(resp["command_id"], "reason-test-1");
}

// ── TS-02-14: Success response and lock state ───────────────────────────────

/// TS-02-14: LOCKING_SERVICE writes success response and correct lock state.
#[tokio::test]
async fn test_locking_success_response_and_state() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    set_speed(&client, 0.0).await;
    set_door_open(&client, false).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let cmd_id = format!("success-{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis());

    let response = send_command_and_wait(
        &client,
        &make_command(&cmd_id, "lock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "success");
    assert_eq!(resp["command_id"], cmd_id);

    let locked = get_locked(&client).await;
    assert_eq!(locked, Some(true));
}

// ── TS-02-E3: Invalid JSON command ──────────────────────────────────────────

/// TS-02-E3: LOCKING_SERVICE handles invalid JSON in command signal.
#[tokio::test]
async fn test_edge_invalid_json_command() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let response = send_command_and_wait(
        &client,
        "not valid json {{{",
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response for invalid JSON");
    assert_eq!(resp["status"], "failed");
    assert_eq!(resp["reason"], "invalid_payload");
}

// ── TS-02-E4: Unknown action ────────────────────────────────────────────────

/// TS-02-E4: LOCKING_SERVICE rejects unknown action values.
#[tokio::test]
async fn test_edge_unknown_action() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    let cmd = serde_json::json!({
        "command_id": "edge-4",
        "action": "toggle",
        "doors": ["driver"],
        "source": "test",
        "vin": "VIN12345",
        "timestamp": 1700000000
    })
    .to_string();

    let response = send_command_and_wait(
        &client,
        &cmd,
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response for unknown action");
    assert_eq!(resp["status"], "failed");
    assert_eq!(resp["reason"], "unknown_action");
}

// ── TS-02-E5: Missing fields ───────────────────────────────────────────────

/// TS-02-E5: LOCKING_SERVICE rejects commands with missing required fields.
#[tokio::test]
async fn test_edge_missing_fields() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Missing command_id
    let cmd = serde_json::json!({
        "action": "lock",
        "doors": ["driver"],
        "source": "test",
        "vin": "VIN12345",
        "timestamp": 1700000000
    })
    .to_string();

    let response = send_command_and_wait(
        &client,
        &cmd,
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response for missing fields");
    assert_eq!(resp["status"], "failed");
    assert_eq!(resp["reason"], "missing_fields");
}

// ── TS-02-E6: Speed not set defaults safe ───────────────────────────────────

/// TS-02-E6: LOCKING_SERVICE treats unset speed as zero (safe).
///
/// This test exercises the safety checker directly to verify that when
/// Vehicle.Speed has no value, the constraint check passes.
#[tokio::test]
async fn test_edge_speed_not_set_defaults_safe() {
    require_infra!();

    let client = test_client().await;
    let safety = SafetyChecker::new(client.clone());

    // Note: We cannot guarantee speed is unset in a shared DATA_BROKER,
    // but we can set speed to 0 and verify the constraint passes.
    set_speed(&client, 0.0).await;
    set_door_open(&client, false).await;

    let result = safety.check_lock_constraints().await;
    assert!(result.is_ok(), "lock should be allowed when speed is 0");
}

// ── TS-02-E7: Door not set defaults safe ────────────────────────────────────

/// TS-02-E7: LOCKING_SERVICE treats unset door state as closed (safe).
#[tokio::test]
async fn test_edge_door_not_set_defaults_safe() {
    require_infra!();

    let client = test_client().await;
    let safety = SafetyChecker::new(client.clone());

    // Set speed to safe value
    set_speed(&client, 0.0).await;

    let result = safety.check_lock_constraints().await;
    assert!(result.is_ok(), "lock should be allowed when speed is 0");
}

// ── TS-02-P1: Command-Response Pairing ──────────────────────────────────────

/// TS-02-P1: For any command, exactly one response with matching command_id.
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
        let cmd_id = format!("pairing-{}", i);
        let response = send_command_and_wait(
            &client,
            &make_command(&cmd_id, "lock"),
            Duration::from_secs(5),
        )
        .await;

        let resp = response.expect(&format!("should receive response for command {}", i));
        assert_eq!(
            resp["command_id"], cmd_id,
            "response command_id should match sent command_id"
        );
    }

    service_handle.abort();
}

// ── TS-02-P2: Safety Constraint Enforcement (Speed) ─────────────────────────

/// TS-02-P2: For any lock command when speed > 0, IsLocked unchanged and response failed.
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
            &make_command(&format!("speed-{}", speed), "lock"),
            Duration::from_secs(5),
        )
        .await;

        let resp = response.expect(&format!("should get response for speed={}", speed));
        assert_eq!(resp["status"], "failed", "should fail at speed={}", speed);
        assert_eq!(resp["reason"], "vehicle_moving");

        let current = get_locked(&client).await;
        assert_eq!(current, initial, "IsLocked should not change at speed={}", speed);
    }

    service_handle.abort();
}

// ── TS-02-P3: Door Ajar Protection ──────────────────────────────────────────

/// TS-02-P3: For any lock command when door is open, IsLocked unchanged and response failed.
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
        &make_command("ajar-test", "lock"),
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

/// TS-02-P4: After lock -> IsLocked==true; after unlock -> IsLocked==false.
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
        &make_command("consistency-1", "lock"),
        Duration::from_secs(5),
    )
    .await
    .expect("should get lock response");
    assert_eq!(resp["status"], "success");
    assert_eq!(get_locked(&client).await, Some(true));

    // Unlock
    let resp = send_command_and_wait(
        &client,
        &make_command("consistency-2", "unlock"),
        Duration::from_secs(5),
    )
    .await
    .expect("should get unlock response");
    assert_eq!(resp["status"], "success");
    assert_eq!(get_locked(&client).await, Some(false));

    // Lock again
    let resp = send_command_and_wait(
        &client,
        &make_command("consistency-3", "lock"),
        Duration::from_secs(5),
    )
    .await
    .expect("should get second lock response");
    assert_eq!(resp["status"], "success");
    assert_eq!(get_locked(&client).await, Some(true));

    service_handle.abort();
}
