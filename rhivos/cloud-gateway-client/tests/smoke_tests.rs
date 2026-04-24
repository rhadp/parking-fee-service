//! Binary-level smoke tests for CLOUD_GATEWAY_CLIENT.
//!
//! These tests spawn the actual `cloud-gateway-client` binary as a subprocess
//! and verify exit codes, stderr output, and observable side effects (e.g.,
//! NATS registration messages).
//!
//! **Prerequisites for `#[ignore]` tests:** Start containers before running:
//! ```sh
//! podman-compose -f deployments/compose.yml up -d
//! ```
//!
//! **Run:** `cargo test -p cloud-gateway-client -- --ignored`
//!
//! Test Specifications:
//! - TS-04-SMOKE-1: Service starts with valid configuration
//! - TS-04-SMOKE-2: Service exits on missing VIN
//! - TS-04-SMOKE-3: Service publishes registration on startup
//! - TS-04-15: NATS reconnection with exponential backoff

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use futures::StreamExt;

const NATS_URL: &str = "nats://localhost:4222";
const DATABROKER_ADDR: &str = "http://localhost:55556";

/// Returns the path to the `cloud-gateway-client` binary.
///
/// Navigates from the test executable's location in `target/debug/deps/`
/// up to `target/debug/` where the main binary is placed by cargo.
fn binary_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().expect("failed to get current exe path");
    path.pop(); // remove test binary name (e.g., smoke_tests-xxxx)
    path.pop(); // remove deps/
    path.push("cloud-gateway-client");
    path
}

// ===========================================================================
// TS-04-SMOKE-1: Service starts with valid configuration
// Validates: [04-REQ-2.1], [04-REQ-3.1]
// ===========================================================================

/// GIVEN NATS container is running on localhost:4222
/// GIVEN DATA_BROKER container is running on localhost:55556
/// GIVEN env VIN="SMOKE-S1-VIN"
/// WHEN CLOUD_GATEWAY_CLIENT binary is executed
/// THEN the process starts without error
///   AND logs contain "connected to NATS"
///   AND logs contain "connected to DATA_BROKER"
#[tokio::test]
#[ignore]
#[serial_test::serial]
async fn ts_04_smoke_1_service_starts_with_valid_config() {
    let mut child = Command::new(binary_path())
        .env("VIN", "SMOKE-S1-VIN")
        .env("NATS_URL", NATS_URL)
        .env("DATABROKER_ADDR", DATABROKER_ADDR)
        .env("RUST_LOG", "info")
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .expect("failed to start cloud-gateway-client");

    // Allow the service to complete its startup sequence.
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Kill the process (it runs indefinitely after successful startup).
    child.kill().expect("failed to kill cloud-gateway-client");
    let output = child
        .wait_with_output()
        .expect("failed to collect process output");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains("connected to NATS"),
        "logs should contain 'connected to NATS', got:\n{stderr}"
    );
    assert!(
        stderr.contains("connected to DATA_BROKER"),
        "logs should contain 'connected to DATA_BROKER', got:\n{stderr}"
    );
}

// ===========================================================================
// TS-04-SMOKE-2: Service exits on missing VIN
// Validates: [04-REQ-1.E1]
// ===========================================================================

/// GIVEN env VIN is not set
/// WHEN CLOUD_GATEWAY_CLIENT binary is executed
/// THEN the process exits with code 1
///   AND stderr contains "VIN"
#[test]
fn ts_04_smoke_2_exits_on_missing_vin() {
    let output = Command::new(binary_path())
        .env_remove("VIN")
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .output()
        .expect("failed to execute cloud-gateway-client");

    assert_eq!(
        output.status.code(),
        Some(1),
        "process should exit with code 1 when VIN is missing"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("VIN"),
        "stderr should mention VIN, got:\n{stderr}"
    );
}

// ===========================================================================
// TS-04-SMOKE-3: Service publishes registration on startup
// Validates: [04-REQ-4.1]
// ===========================================================================

/// GIVEN NATS subscriber is listening on "vehicles.SMOKE-S3-VIN.status"
/// GIVEN env VIN="SMOKE-S3-VIN"
/// WHEN CLOUD_GATEWAY_CLIENT binary is executed
/// THEN within 5 seconds, a registration message is received
///   AND the JSON contains "vin":"SMOKE-S3-VIN"
///   AND the JSON contains "status":"online"
#[tokio::test]
#[ignore]
#[serial_test::serial]
async fn ts_04_smoke_3_registration_on_startup() {
    let vin = "SMOKE-S3-VIN";

    // Subscribe to the status subject BEFORE starting the binary so we
    // do not miss the fire-and-forget registration message.
    let nats = async_nats::connect(NATS_URL)
        .await
        .expect("failed to connect to NATS for test subscription");
    let mut sub = nats
        .subscribe(format!("vehicles.{vin}.status"))
        .await
        .expect("failed to subscribe to status subject");

    // Allow the subscription to propagate to the NATS server.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Start the binary.
    let mut child = Command::new(binary_path())
        .env("VIN", vin)
        .env("NATS_URL", NATS_URL)
        .env("DATABROKER_ADDR", DATABROKER_ADDR)
        .env("RUST_LOG", "info")
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .expect("failed to start cloud-gateway-client");

    // Wait for the registration message (up to 5 seconds).
    let msg = tokio::time::timeout(Duration::from_secs(5), sub.next()).await;

    // Clean up: kill the process.
    let _ = child.kill();
    let _ = child.wait();

    let msg = msg
        .expect("timeout: no registration message received within 5s")
        .expect("NATS subscription ended unexpectedly");

    let payload: serde_json::Value =
        serde_json::from_slice(&msg.payload).expect("registration message should be valid JSON");

    assert_eq!(
        payload["vin"].as_str(),
        Some(vin),
        "registration should contain the correct VIN"
    );
    assert_eq!(
        payload["status"].as_str(),
        Some("online"),
        "registration should have status 'online'"
    );
    assert!(
        payload.get("timestamp").is_some(),
        "registration should contain a timestamp"
    );
}

// ===========================================================================
// TS-04-15: NATS reconnection with exponential backoff
// Validates: [04-REQ-2.2], [04-REQ-2.E1]
// ===========================================================================

/// GIVEN NATS server is not running (port 19222 is unreachable)
/// WHEN CLOUD_GATEWAY_CLIENT is started with NATS_URL pointing to the
///   unreachable address
/// THEN the service retries with exponential backoff (delays: 1s, 2s, 4s, 8s)
///   AND after 5 failed attempts, the service exits with code 1
#[tokio::test]
#[ignore]
#[serial_test::serial]
async fn ts_04_15_nats_reconnection_backoff() {
    let start = Instant::now();

    // Use an unreachable NATS address. Port 19222 on localhost should have
    // nothing listening, causing immediate "connection refused" errors.
    let output = tokio::process::Command::new(binary_path())
        .env("VIN", "BACKOFF-VIN")
        .env("NATS_URL", "nats://127.0.0.1:19222")
        .env("DATABROKER_ADDR", DATABROKER_ADDR)
        .env("RUST_LOG", "info")
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .output()
        .await
        .expect("failed to execute cloud-gateway-client");

    let elapsed = start.elapsed();

    assert_eq!(
        output.status.code(),
        Some(1),
        "process should exit with code 1 after NATS retries exhausted"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify the service logged connection failure messages.
    assert!(
        stderr.contains("NATS server unreachable"),
        "stderr should log 'NATS server unreachable' after retries exhausted, got:\n{stderr}"
    );

    // The backoff delays are [1, 2, 4, 8] = 15s total minimum.
    // With near-instant connection-refused errors, the process should
    // take approximately 15 seconds. Allow generous tolerance for CI.
    assert!(
        elapsed >= Duration::from_secs(13),
        "process should take at least 13s due to backoff delays, took {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(45),
        "process should exit within 45s, took {elapsed:?}"
    );
}
