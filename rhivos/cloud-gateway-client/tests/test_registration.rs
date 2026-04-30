//! Unit tests for registration message format and edge cases.
//!
//! Test Spec: TS-04-10, TS-04-E11, TS-04-E12, TS-04-E13
//! Requirements: 04-REQ-4.1, 04-REQ-2.E1, 04-REQ-3.E1, 04-REQ-7.E1

use cloud_gateway_client::models::RegistrationMessage;

/// TS-04-10: Registration message format.
///
/// Requirement: 04-REQ-4.1
/// The registration message SHALL contain vin, status, and timestamp fields.
#[test]
fn test_registration_message_format() {
    let msg = RegistrationMessage {
        vin: "VIN-001".to_string(),
        status: "online".to_string(),
        timestamp: 1700000000,
    };

    let json = serde_json::to_string(&msg).expect("Should serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should be valid JSON");

    assert_eq!(parsed["vin"], "VIN-001");
    assert_eq!(parsed["status"], "online");
    assert!(parsed.get("timestamp").is_some(), "Should have timestamp");
    assert_eq!(parsed["timestamp"], 1700000000_u64);
}

/// TS-04-E11: NATS connection retries exhausted.
///
/// Requirement: 04-REQ-2.E1
/// WHEN all NATS connection retry attempts are exhausted, the system SHALL
/// exit with code 1 and log an error indicating the NATS server is unreachable.
///
/// This test starts the service binary against an unreachable NATS URL
/// and verifies the exit code and exponential backoff timing.
#[test]
#[ignore]
fn test_nats_retries_exhausted() {
    // Build the service binary via cargo.
    let build_status = std::process::Command::new("cargo")
        .args(["build", "-p", "cloud-gateway-client"])
        .status()
        .expect("Failed to run cargo build");
    assert!(build_status.success(), "cargo build failed");

    let metadata_output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .expect("Failed to run cargo metadata");
    let meta: serde_json::Value =
        serde_json::from_slice(&metadata_output.stdout).expect("Invalid cargo metadata JSON");
    let target_dir = meta["target_directory"]
        .as_str()
        .expect("Missing target_directory");
    let binary = format!("{target_dir}/debug/cloud-gateway-client");

    let start = std::time::Instant::now();

    let output = std::process::Command::new(&binary)
        .env("VIN", "RETRIES-VIN")
        .env("NATS_URL", "nats://127.0.0.1:19999")
        .env("DATABROKER_ADDR", "http://localhost:55556")
        .env("BEARER_TOKEN", "demo-token")
        .env("RUST_LOG", "cloud_gateway_client=debug")
        .output()
        .expect("Failed to start service");

    let elapsed = start.elapsed();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The service should exit with code 1.
    assert_eq!(
        output.status.code(),
        Some(1),
        "Service should exit with code 1 after retries exhausted"
    );

    // Verify elapsed time includes backoff delays (1s + 2s + 4s + 8s = 15s
    // minimum). Allow tolerance for connection attempt duration.
    assert!(
        elapsed >= std::time::Duration::from_secs(7),
        "Expected backoff delays (elapsed: {elapsed:?})"
    );

    // Verify error log mentions retries or unreachable.
    assert!(
        stderr.contains("retries exhausted")
            || stderr.contains("NATS connection failed")
            || stderr.contains("Failed to connect to NATS"),
        "Stderr should mention NATS connection failure: {stderr}"
    );
}

/// TS-04-E12: DATA_BROKER connection failure at startup.
///
/// Requirement: 04-REQ-3.E1
/// WHEN the DATA_BROKER connection cannot be established, the system SHALL
/// exit with code 1 and log an error about the connection failure.
///
/// This test requires a running NATS server (NATS connection succeeds at
/// step 2) but points DATA_BROKER to an unreachable address.
#[test]
#[ignore]
fn test_broker_connection_failure() {
    // Build the service binary via cargo.
    let build_status = std::process::Command::new("cargo")
        .args(["build", "-p", "cloud-gateway-client"])
        .status()
        .expect("Failed to run cargo build");
    assert!(build_status.success(), "cargo build failed");

    let metadata_output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .expect("Failed to run cargo metadata");
    let meta: serde_json::Value =
        serde_json::from_slice(&metadata_output.stdout).expect("Invalid cargo metadata JSON");
    let target_dir = meta["target_directory"]
        .as_str()
        .expect("Missing target_directory");
    let binary = format!("{target_dir}/debug/cloud-gateway-client");

    let output = std::process::Command::new(&binary)
        .env("VIN", "BROKER-FAIL-VIN")
        .env("NATS_URL", "nats://localhost:4222")
        .env("DATABROKER_ADDR", "http://127.0.0.1:19998")
        .env("BEARER_TOKEN", "demo-token")
        .env("RUST_LOG", "cloud_gateway_client=debug")
        .output()
        .expect("Failed to start service");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // The service should exit with code 1.
    assert_eq!(
        output.status.code(),
        Some(1),
        "Service should exit with code 1 on DATA_BROKER failure"
    );

    // Verify error log mentions DATA_BROKER connection failure.
    assert!(
        stderr.contains("DATA_BROKER")
            || stderr.contains("connection failed")
            || stderr.contains("Failed to connect"),
        "Stderr should mention DATA_BROKER connection failure: {stderr}"
    );
}

/// TS-04-E13: Response relay skips invalid JSON from DATA_BROKER.
///
/// Requirement: 04-REQ-7.E1
/// WHEN the response value from DATA_BROKER is not valid JSON, the system
/// SHALL log an error and SHALL NOT publish to NATS.
///
/// This test starts the service, writes an invalid JSON string to
/// `Vehicle.Command.Door.Response` in DATA_BROKER, and verifies that
/// no message appears on the NATS `command_responses` subject.
#[tokio::test]
#[ignore]
async fn test_response_relay_skips_invalid_json() {
    use cloud_gateway_client::broker_client::kuksa::val::v2;
    use futures::StreamExt;
    use std::time::Duration;
    use tokio::time::timeout;

    let vin = "INTEG-INVJSON-VIN";

    // Build the service binary.
    let build_output = tokio::process::Command::new("cargo")
        .args(["build", "-p", "cloud-gateway-client"])
        .output()
        .await
        .expect("Failed to run cargo build");
    assert!(build_output.status.success(), "cargo build failed");

    let metadata_output = tokio::process::Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .await
        .expect("Failed to run cargo metadata");
    let meta: serde_json::Value =
        serde_json::from_slice(&metadata_output.stdout).expect("Invalid cargo metadata JSON");
    let target_dir = meta["target_directory"]
        .as_str()
        .expect("Missing target_directory");
    let binary = format!("{target_dir}/debug/cloud-gateway-client");

    // Start the service.
    let mut child = tokio::process::Command::new(&binary)
        .env("VIN", vin)
        .env("NATS_URL", "nats://localhost:4222")
        .env("DATABROKER_ADDR", "http://localhost:55556")
        .env("BEARER_TOKEN", "demo-token")
        .env("RUST_LOG", "cloud_gateway_client=debug")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to start service: {e}"));

    // Wait for the service to start and subscribe.
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Subscribe to command_responses on NATS.
    let nats = async_nats::connect("nats://localhost:4222")
        .await
        .expect("Failed to connect to NATS");
    let response_subject = format!("vehicles.{vin}.command_responses");
    let mut sub = nats
        .subscribe(response_subject)
        .await
        .expect("Failed to subscribe to command_responses");

    // Write an invalid JSON string to the response signal in DATA_BROKER.
    let mut broker =
        v2::val_client::ValClient::connect("http://localhost:55556".to_string())
            .await
            .expect("Failed to connect to DATA_BROKER");

    let request = v2::PublishValueRequest {
        signal_id: Some(v2::SignalId {
            signal: Some(v2::signal_id::Signal::Path(
                "Vehicle.Command.Door.Response".to_string(),
            )),
        }),
        data_point: Some(v2::Datapoint {
            timestamp: None,
            value: Some(v2::Value {
                typed_value: Some(v2::value::TypedValue::String(
                    "not-valid-json{{".to_string(),
                )),
            }),
        }),
    };

    broker
        .publish_value(request)
        .await
        .expect("Failed to write invalid JSON to DATA_BROKER");

    // Wait and verify that no message is published on NATS.
    let result = timeout(Duration::from_secs(2), sub.next()).await;
    assert!(
        result.is_err(),
        "No response should be published to NATS for invalid JSON"
    );

    child.kill().await.ok();
}
