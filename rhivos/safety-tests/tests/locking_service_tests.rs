//! LOCKING_SERVICE integration tests
//!
//! These tests verify LOCKING_SERVICE command subscription, lock/unlock
//! execution, and safety constraint enforcement.
//!
//! Test Spec: TS-02-6, TS-02-8, TS-02-9, TS-02-10, TS-02-11, TS-02-12,
//!            TS-02-13, TS-02-14

use std::time::Duration;

use databroker_client::{DataValue, DatabrokerClient};
use locking_service::command::{self, LockAction, ParseResult};
use locking_service::safety::SafetyChecker;
use locking_service::service::signals;
use tokio_stream::StreamExt;

/// Helper: check if infrastructure is available.
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
            eprintln!("SKIP: infrastructure not available (run `make infra-up`)");
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

// ── TS-02-6: Subscribe to commands ──────────────────────────────────────────

/// TS-02-6: LOCKING_SERVICE subscribes to command signals (02-REQ-2.1)
#[tokio::test]
async fn test_locking_subscribes_to_commands() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    set_speed(&client, 0.0).await;
    set_door_open(&client, false).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let response = send_command_and_wait(
        &client,
        &make_command("sub-test-s1", "lock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["command_id"], "sub-test-s1");
}

// ── TS-02-8: Execute lock ───────────────────────────────────────────────────

/// TS-02-8: LOCKING_SERVICE executes lock action (02-REQ-2.3)
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
        &make_command("lock-test-s1", "lock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "success");
    assert_eq!(get_locked(&client).await, Some(true));
}

// ── TS-02-9: Execute unlock ─────────────────────────────────────────────────

/// TS-02-9: LOCKING_SERVICE executes unlock action (02-REQ-2.4)
#[tokio::test]
async fn test_locking_executes_unlock() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    set_speed(&client, 0.0).await;
    set_door_open(&client, false).await;
    client
        .set_value(signals::IS_LOCKED, DataValue::Bool(true))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let response = send_command_and_wait(
        &client,
        &make_command("unlock-test-s1", "unlock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "success");
    assert_eq!(get_locked(&client).await, Some(false));
}

// ── TS-02-10: Reject lock when vehicle moving ───────────────────────────────

/// TS-02-10: LOCKING_SERVICE rejects lock when vehicle moving (02-REQ-3.1)
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
        &make_command("moving-lock-s1", "lock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "failed");
    assert_eq!(resp["reason"], "vehicle_moving");
    assert_eq!(get_locked(&client).await, initial_locked);
}

// ── TS-02-11: Reject lock when door open ────────────────────────────────────

/// TS-02-11: LOCKING_SERVICE rejects lock when door is open (02-REQ-3.2)
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
        &make_command("door-open-s1", "lock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "failed");
    assert_eq!(resp["reason"], "door_open");
    assert_eq!(get_locked(&client).await, initial_locked);
}

// ── TS-02-12: Reject unlock when vehicle moving ─────────────────────────────

/// TS-02-12: LOCKING_SERVICE rejects unlock when vehicle moving (02-REQ-3.3)
#[tokio::test]
async fn test_locking_rejects_unlock_vehicle_moving() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    client
        .set_value(signals::IS_LOCKED, DataValue::Bool(true))
        .await
        .unwrap();
    set_speed(&client, 15.0).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let response = send_command_and_wait(
        &client,
        &make_command("moving-unlock-s1", "unlock"),
        Duration::from_secs(5),
    )
    .await;

    service_handle.abort();

    let resp = response.expect("should receive a response");
    assert_eq!(resp["status"], "failed");
    assert_eq!(resp["reason"], "vehicle_moving");
    assert_eq!(get_locked(&client).await, Some(true));
}

// ── TS-02-13: Failure response includes reason ──────────────────────────────

/// TS-02-13: LOCKING_SERVICE writes failure response with reason (02-REQ-3.4)
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
        &make_command("reason-test-s1", "lock"),
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
    assert_eq!(resp["command_id"], "reason-test-s1");
}

// ── TS-02-14: Success response and lock state ───────────────────────────────

/// TS-02-14: LOCKING_SERVICE writes success response and lock state (02-REQ-3.5)
#[tokio::test]
async fn test_locking_success_response_and_state() {
    require_infra!();

    let client = test_client().await;
    let service_handle = spawn_locking_service(client.clone());
    tokio::time::sleep(Duration::from_millis(500)).await;

    set_speed(&client, 0.0).await;
    set_door_open(&client, false).await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let cmd_id = format!(
        "success-s-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

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
    assert_eq!(get_locked(&client).await, Some(true));
}
