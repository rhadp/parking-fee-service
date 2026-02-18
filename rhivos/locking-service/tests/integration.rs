//! Integration tests for the LOCKING_SERVICE end-to-end flow.
//!
//! These tests exercise the full lock/unlock pipeline:
//!
//!   KuksaClient (acting as mock-sensors)
//!     → DATA_BROKER (Kuksa Databroker)
//!       → LOCKING_SERVICE (lock_handler)
//!         → DATA_BROKER (IsLocked, LockResult)
//!
//! # Prerequisites
//!
//! - A running Kuksa Databroker (via `make infra-up`).
//! - Tests are `#[ignore]` by default; run with `cargo test -- --ignored`.
//!
//! # Requirements
//!
//! - 02-REQ-7.1: Lock happy path (safe conditions → IsLocked = true, LockResult = SUCCESS)
//! - 02-REQ-7.2: Speed rejection (speed >= 1.0 → IsLocked unchanged, LockResult = REJECTED_SPEED)
//! - 02-REQ-7.3: Door-ajar rejection (door open → IsLocked unchanged, LockResult = REJECTED_DOOR_OPEN)
//! - 02-REQ-7.4: Unlock happy path (safe conditions → IsLocked = false, LockResult = SUCCESS)
//! - 02-REQ-7.E1: Tests skip cleanly when DATA_BROKER is unavailable.
//! - 02-REQ-3.4: Unlock with door open is allowed (no door-ajar check for unlock).

use std::time::Duration;

use parking_proto::kuksa_client::KuksaClient;
use parking_proto::signals;
use tokio_stream::StreamExt;

use locking_service::config::Config;
use locking_service::lock_handler::{run_lock_handler, KuksaDataBroker};

/// Default timeout for waiting on locking-service to process a command.
const PROCESSING_TIMEOUT: Duration = Duration::from_secs(5);

/// Short delay to allow the subscription stream to stabilise before sending commands.
const SUBSCRIBE_SETTLE: Duration = Duration::from_millis(500);

/// Databroker address — read from `DATABROKER_ADDR` env var or default.
fn databroker_addr() -> String {
    std::env::var("DATABROKER_ADDR").unwrap_or_else(|_| "http://localhost:55555".to_string())
}

/// Try to connect to the Kuksa Databroker. If the connection fails, the test
/// is skipped with a clear message (02-REQ-7.E1).
async fn connect_or_skip() -> KuksaClient {
    let addr = databroker_addr();
    match KuksaClient::connect(&addr).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!(
                "SKIPPING: Kuksa Databroker unavailable at '{}': {}. \
                 Run `make infra-up` to start the infrastructure.",
                addr, e
            );
            panic!(
                "DATA_BROKER unavailable at '{}': {}. Run `make infra-up`.",
                addr, e
            );
        }
    }
}

/// Reset vehicle state signals in the databroker to a known baseline.
///
/// Sets safe defaults: speed = 0, door = closed, not locked.
/// Does NOT reset LockResult (it has `allowed` constraints in the VSS overlay).
async fn reset_signals(client: &KuksaClient) {
    client
        .set_f32(signals::SPEED, 0.0)
        .await
        .expect("failed to reset speed");
    client
        .set_bool(signals::DOOR_IS_OPEN, false)
        .await
        .expect("failed to reset door state");
    client
        .set_bool(signals::DOOR_IS_LOCKED, false)
        .await
        .expect("failed to reset IsLocked");
}

/// Spawn the locking-service handler as a background Tokio task.
///
/// Returns a `JoinHandle` that can be used to abort the handler when the test
/// is done. The handler subscribes to `Vehicle.Command.Door.Lock` and processes
/// commands through safety validation.
fn spawn_lock_handler(
    client: KuksaClient,
    config: Config,
) -> tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
    tokio::spawn(async move {
        let broker = KuksaDataBroker::new(client.clone());
        run_lock_handler(&client, &broker, &config)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("lock handler error: {e}").into()
            })
    })
}

/// Send a lock command and wait for the locking-service to write a LockResult.
///
/// Uses a subscription on the LockResult signal to detect when the handler has
/// processed the command. The subscription may deliver the current value first
/// (from a prior test or handler), so we:
///
/// 1. Subscribe to LockResult.
/// 2. Drain any initial/current value from the stream.
/// 3. Send the lock command.
/// 4. Wait for the next (fresh) value from the stream.
async fn send_command_and_wait(client: &KuksaClient, lock: bool) -> String {
    // Subscribe to LockResult *before* sending the command so we don't miss it.
    let mut result_stream = client
        .subscribe_string(signals::LOCK_RESULT)
        .await
        .expect("failed to subscribe to LockResult");

    // Drain the initial/current value if present. Kuksa may deliver the current
    // signal value upon subscription establishment.
    let _ = tokio::time::timeout(Duration::from_millis(200), result_stream.next()).await;

    // Write the lock command.
    client
        .set_bool(signals::COMMAND_DOOR_LOCK, lock)
        .await
        .expect("failed to write lock command");

    // Wait for the handler to process and write a fresh result to LockResult.
    let deadline = tokio::time::Instant::now() + PROCESSING_TIMEOUT;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            panic!(
                "timed out waiting for LockResult ({}s). Is the locking-service handler running?",
                PROCESSING_TIMEOUT.as_secs()
            );
        }

        match tokio::time::timeout(remaining, result_stream.next()).await {
            Ok(Some(Ok(result))) => return result,
            Ok(Some(Err(e))) => panic!("error reading LockResult from subscription: {e}"),
            Ok(None) => panic!("LockResult subscription stream ended unexpectedly"),
            Err(_) => panic!(
                "timed out waiting for LockResult ({}s)",
                PROCESSING_TIMEOUT.as_secs()
            ),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Task 8.2: Happy path — lock and unlock
// Requirements: 02-REQ-7.1, 02-REQ-7.4
// Property 1: Command-Lock Invariant
// ═══════════════════════════════════════════════════════════════════════════

/// Test: safe conditions + lock command → IsLocked = true, LockResult = "SUCCESS".
///
/// Requirements: 02-REQ-7.1
/// Property: Command-Lock Invariant
#[tokio::test]
#[ignore]
async fn integration_lock_happy_path() {
    let client = connect_or_skip().await;
    reset_signals(&client).await;

    let config = Config {
        databroker_addr: databroker_addr(),
        max_speed_kmh: 1.0,
    };

    let handler = spawn_lock_handler(client.clone(), config);

    // Allow subscription to establish.
    tokio::time::sleep(SUBSCRIBE_SETTLE).await;

    // Set safe conditions: speed = 0, door = closed.
    client.set_f32(signals::SPEED, 0.0).await.unwrap();
    client.set_bool(signals::DOOR_IS_OPEN, false).await.unwrap();

    // Send lock command and wait for result.
    let result = send_command_and_wait(&client, true).await;

    // Verify results.
    assert_eq!(
        result, "SUCCESS",
        "expected LockResult = SUCCESS for safe lock"
    );

    let is_locked = client
        .get_bool(signals::DOOR_IS_LOCKED)
        .await
        .expect("failed to read IsLocked");
    assert_eq!(
        is_locked,
        Some(true),
        "expected IsLocked = true after successful lock"
    );

    // Clean up.
    handler.abort();
}

/// Test: safe conditions + unlock command → IsLocked = false, LockResult = "SUCCESS".
///
/// Requirements: 02-REQ-7.4
/// Property: Command-Lock Invariant
#[tokio::test]
#[ignore]
async fn integration_unlock_happy_path() {
    let client = connect_or_skip().await;
    reset_signals(&client).await;

    // Set ALL conditions BEFORE spawning the handler to avoid races with
    // stale subscription values.
    client.set_f32(signals::SPEED, 0.0).await.unwrap();
    client.set_bool(signals::DOOR_IS_OPEN, false).await.unwrap();
    // Pre-set the door as locked so we can verify the unlock changes it.
    client
        .set_bool(signals::DOOR_IS_LOCKED, true)
        .await
        .unwrap();

    let config = Config {
        databroker_addr: databroker_addr(),
        max_speed_kmh: 1.0,
    };

    let handler = spawn_lock_handler(client.clone(), config);
    tokio::time::sleep(SUBSCRIBE_SETTLE).await;

    // Send unlock command.
    let result = send_command_and_wait(&client, false).await;

    // Verify results.
    assert_eq!(
        result, "SUCCESS",
        "expected LockResult = SUCCESS for safe unlock"
    );

    let is_locked = client
        .get_bool(signals::DOOR_IS_LOCKED)
        .await
        .expect("failed to read IsLocked");
    assert_eq!(
        is_locked,
        Some(false),
        "expected IsLocked = false after successful unlock"
    );

    handler.abort();
}

// ═══════════════════════════════════════════════════════════════════════════
// Task 8.3: Safety rejections
// Requirements: 02-REQ-7.2, 02-REQ-7.3
// Property 2: Safety Rejection Guarantee
// Property 3: Result Completeness
// ═══════════════════════════════════════════════════════════════════════════

/// Test: speed >= 1.0 + lock command → IsLocked unchanged, LockResult = "REJECTED_SPEED".
///
/// Requirements: 02-REQ-7.2
/// Property: Safety Rejection Guarantee
#[tokio::test]
#[ignore]
async fn integration_lock_rejected_speed() {
    let client = connect_or_skip().await;
    reset_signals(&client).await;

    let config = Config {
        databroker_addr: databroker_addr(),
        max_speed_kmh: 1.0,
    };

    // Set conditions BEFORE spawning the handler.
    client.set_f32(signals::SPEED, 50.0).await.unwrap();
    client.set_bool(signals::DOOR_IS_OPEN, false).await.unwrap();

    let handler = spawn_lock_handler(client.clone(), config);
    tokio::time::sleep(SUBSCRIBE_SETTLE).await;

    // Record the current IsLocked state (should be false from reset).
    let is_locked_before = client
        .get_bool(signals::DOOR_IS_LOCKED)
        .await
        .expect("failed to read IsLocked before command");

    // Send lock command.
    let result = send_command_and_wait(&client, true).await;

    // Verify rejection.
    assert_eq!(
        result, "REJECTED_SPEED",
        "expected LockResult = REJECTED_SPEED for high speed"
    );

    let is_locked_after = client
        .get_bool(signals::DOOR_IS_LOCKED)
        .await
        .expect("failed to read IsLocked after command");
    assert_eq!(
        is_locked_before, is_locked_after,
        "IsLocked must NOT change on speed rejection"
    );

    handler.abort();
}

/// Test: door open + lock command → IsLocked unchanged, LockResult = "REJECTED_DOOR_OPEN".
///
/// Requirements: 02-REQ-7.3
/// Property: Safety Rejection Guarantee
#[tokio::test]
#[ignore]
async fn integration_lock_rejected_door_open() {
    let client = connect_or_skip().await;
    reset_signals(&client).await;

    let config = Config {
        databroker_addr: databroker_addr(),
        max_speed_kmh: 1.0,
    };

    // Set conditions BEFORE spawning the handler.
    client.set_f32(signals::SPEED, 0.0).await.unwrap();
    client.set_bool(signals::DOOR_IS_OPEN, true).await.unwrap();

    let handler = spawn_lock_handler(client.clone(), config);
    tokio::time::sleep(SUBSCRIBE_SETTLE).await;

    // Record the current IsLocked state.
    let is_locked_before = client
        .get_bool(signals::DOOR_IS_LOCKED)
        .await
        .expect("failed to read IsLocked before command");

    // Send lock command.
    let result = send_command_and_wait(&client, true).await;

    // Verify rejection.
    assert_eq!(
        result, "REJECTED_DOOR_OPEN",
        "expected LockResult = REJECTED_DOOR_OPEN for door ajar"
    );

    let is_locked_after = client
        .get_bool(signals::DOOR_IS_LOCKED)
        .await
        .expect("failed to read IsLocked after command");
    assert_eq!(
        is_locked_before, is_locked_after,
        "IsLocked must NOT change on door-open rejection"
    );

    handler.abort();
}

// ═══════════════════════════════════════════════════════════════════════════
// Task 8.4: Unlock with door open (allowed)
// Requirements: 02-REQ-3.4
// ═══════════════════════════════════════════════════════════════════════════

/// Test: door open + unlock command → IsLocked = false, LockResult = "SUCCESS".
///
/// Validates that the door-ajar check applies only to lock commands, not unlock.
///
/// Requirements: 02-REQ-3.4
#[tokio::test]
#[ignore]
async fn integration_unlock_with_door_open_succeeds() {
    let client = connect_or_skip().await;
    reset_signals(&client).await;

    // Set ALL conditions BEFORE spawning the handler.
    client.set_f32(signals::SPEED, 0.0).await.unwrap();
    client.set_bool(signals::DOOR_IS_OPEN, true).await.unwrap();
    // Pre-set the door as locked.
    client
        .set_bool(signals::DOOR_IS_LOCKED, true)
        .await
        .unwrap();

    let config = Config {
        databroker_addr: databroker_addr(),
        max_speed_kmh: 1.0,
    };

    let handler = spawn_lock_handler(client.clone(), config);
    tokio::time::sleep(SUBSCRIBE_SETTLE).await;

    // Send unlock command (should succeed despite door open).
    let result = send_command_and_wait(&client, false).await;

    // Verify success.
    assert_eq!(
        result, "SUCCESS",
        "expected LockResult = SUCCESS for unlock with door open (door-ajar is lock-only)"
    );

    let is_locked = client
        .get_bool(signals::DOOR_IS_LOCKED)
        .await
        .expect("failed to read IsLocked");
    assert_eq!(
        is_locked,
        Some(false),
        "expected IsLocked = false after unlock with door open"
    );

    handler.abort();
}

// ═══════════════════════════════════════════════════════════════════════════
// Additional coverage: unlock with high speed is also rejected
// ═══════════════════════════════════════════════════════════════════════════

/// Test: high speed + unlock command → IsLocked unchanged, LockResult = "REJECTED_SPEED".
///
/// Validates that speed rejection applies to both lock and unlock commands.
///
/// Requirements: 02-REQ-3.3
#[tokio::test]
#[ignore]
async fn integration_unlock_rejected_speed() {
    let client = connect_or_skip().await;
    reset_signals(&client).await;

    // Set the unsafe conditions BEFORE spawning the handler so that if the
    // handler processes a stale command from the subscription stream, it will
    // already see the unsafe speed.
    client.set_f32(signals::SPEED, 50.0).await.unwrap();
    client.set_bool(signals::DOOR_IS_OPEN, false).await.unwrap();
    client
        .set_bool(signals::DOOR_IS_LOCKED, true)
        .await
        .unwrap();

    let config = Config {
        databroker_addr: databroker_addr(),
        max_speed_kmh: 1.0,
    };

    let handler = spawn_lock_handler(client.clone(), config);
    tokio::time::sleep(SUBSCRIBE_SETTLE).await;

    // Send unlock command.
    let result = send_command_and_wait(&client, false).await;

    // Verify rejection.
    assert_eq!(
        result, "REJECTED_SPEED",
        "expected LockResult = REJECTED_SPEED for unlock at high speed"
    );

    // IsLocked should remain true (not unlocked).
    let is_locked = client
        .get_bool(signals::DOOR_IS_LOCKED)
        .await
        .expect("failed to read IsLocked");
    assert_eq!(
        is_locked,
        Some(true),
        "IsLocked must NOT change on speed rejection for unlock"
    );

    handler.abort();
}
