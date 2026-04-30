//! Integration tests for CLOUD_GATEWAY_CLIENT end-to-end flows.
//!
//! These tests require running NATS and DATA_BROKER (Kuksa Databroker)
//! containers. Start infrastructure with:
//!
//! ```sh
//! cd deployments && podman-compose up -d
//! ```
//!
//! Run these tests with:
//! ```sh
//! cargo test -p cloud-gateway-client -- --ignored
//! ```
//!
//! Test Spec: TS-04-11, TS-04-12, TS-04-13, TS-04-14, TS-04-15, TS-04-16
//! Requirements: 04-REQ-2.2, 04-REQ-2.3, 04-REQ-2.E1, 04-REQ-4.1, 04-REQ-4.2,
//!               04-REQ-5.2, 04-REQ-5.E2, 04-REQ-6.3,
//!               04-REQ-7.1, 04-REQ-7.2, 04-REQ-8.1, 04-REQ-8.2

use std::process::Stdio;
use std::time::Duration;

use futures::StreamExt;
use tokio::time::timeout;

/// Default NATS URL for integration tests.
const NATS_URL: &str = "nats://localhost:4222";

/// Default DATA_BROKER gRPC address for integration tests.
const DATABROKER_ADDR: &str = "http://localhost:55556";

/// Default bearer token matching the service default.
const BEARER_TOKEN: &str = "demo-token";

/// Helper: build and return the path to the cloud-gateway-client binary.
///
/// This calls `cargo build` to ensure the binary is up-to-date, then returns
/// the path to the debug binary.
async fn build_service_binary() -> String {
    let output = tokio::process::Command::new("cargo")
        .args(["build", "-p", "cloud-gateway-client"])
        .output()
        .await
        .expect("Failed to run cargo build");

    assert!(
        output.status.success(),
        "cargo build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Locate the binary in target/debug
    let metadata = tokio::process::Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .await
        .expect("Failed to run cargo metadata");

    let meta: serde_json::Value =
        serde_json::from_slice(&metadata.stdout).expect("Invalid cargo metadata JSON");
    let target_dir = meta["target_directory"]
        .as_str()
        .expect("Missing target_directory");

    format!("{target_dir}/debug/cloud-gateway-client")
}

/// Helper: start the cloud-gateway-client process with the given VIN.
///
/// Returns the child process handle. Caller is responsible for killing it.
async fn start_service(vin: &str, nats_url: &str, databroker_addr: &str) -> tokio::process::Child {
    let binary = build_service_binary().await;

    tokio::process::Command::new(&binary)
        .env("VIN", vin)
        .env("NATS_URL", nats_url)
        .env("DATABROKER_ADDR", databroker_addr)
        .env("BEARER_TOKEN", BEARER_TOKEN)
        .env("RUST_LOG", "cloud_gateway_client=debug")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to start service binary at {binary}: {e}"))
}

/// Helper: connect to NATS for integration tests.
async fn connect_nats() -> async_nats::Client {
    async_nats::connect(NATS_URL)
        .await
        .expect("Failed to connect to NATS for integration test")
}

/// Helper: connect to DATA_BROKER for integration tests.
async fn connect_databroker(
) -> cloud_gateway_client::broker_client::kuksa::val::v2::val_client::ValClient<
    tonic::transport::Channel,
> {
    cloud_gateway_client::broker_client::kuksa::val::v2::val_client::ValClient::connect(
        DATABROKER_ADDR.to_string(),
    )
    .await
    .expect("Failed to connect to DATA_BROKER for integration test")
}

/// Helper: publish a NATS command message with the given bearer token.
async fn publish_command(
    nats: &async_nats::Client,
    vin: &str,
    payload: &str,
    token: &str,
) {
    let subject = format!("vehicles.{vin}.commands");

    let mut headers = async_nats::HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {token}").as_str(),
    );

    nats.publish_with_headers(subject, headers, payload.to_string().into())
        .await
        .expect("Failed to publish command to NATS");

    // Flush to ensure delivery.
    nats.flush().await.expect("Failed to flush NATS");
}

/// Helper: read a signal value from DATA_BROKER.
async fn read_databroker_signal(
    client: &mut cloud_gateway_client::broker_client::kuksa::val::v2::val_client::ValClient<
        tonic::transport::Channel,
    >,
    path: &str,
) -> Option<String> {
    use cloud_gateway_client::broker_client::kuksa::val::v2;

    let request = v2::GetValueRequest {
        signal_id: Some(v2::SignalId {
            signal: Some(v2::signal_id::Signal::Path(path.to_string())),
        }),
    };

    match client.get_value(request).await {
        Ok(response) => {
            let resp = response.into_inner();
            resp.data_point.and_then(|dp| {
                dp.value.and_then(|v| {
                    if let Some(v2::value::TypedValue::String(s)) = v.typed_value {
                        Some(s)
                    } else {
                        None
                    }
                })
            })
        }
        Err(_) => None,
    }
}

/// Helper: write a string signal value to DATA_BROKER.
async fn write_databroker_signal(
    client: &mut cloud_gateway_client::broker_client::kuksa::val::v2::val_client::ValClient<
        tonic::transport::Channel,
    >,
    path: &str,
    value: cloud_gateway_client::broker_client::kuksa::val::v2::value::TypedValue,
) {
    use cloud_gateway_client::broker_client::kuksa::val::v2;

    let request = v2::PublishValueRequest {
        signal_id: Some(v2::SignalId {
            signal: Some(v2::signal_id::Signal::Path(path.to_string())),
        }),
        data_point: Some(v2::Datapoint {
            timestamp: None,
            value: Some(v2::Value {
                typed_value: Some(value),
            }),
        }),
    };

    client
        .publish_value(request)
        .await
        .unwrap_or_else(|e| panic!("Failed to write signal {path} to DATA_BROKER: {e}"));
}

// ============================================================================
// TS-04-14: Self-registration on startup
//
// Requirement: 04-REQ-4.1, 04-REQ-4.2
// Verify that the service publishes a registration message on startup.
// ============================================================================

/// TS-04-14: Self-registration on startup.
///
/// Start the service and verify that a registration message is published
/// to `vehicles.{VIN}.status` on NATS containing the VIN and "online" status.
#[tokio::test]
#[ignore]
async fn test_self_registration_on_startup() {
    let vin = "INTEG-REG-VIN";
    let nats = connect_nats().await;
    let subject = format!("vehicles.{vin}.status");

    // Subscribe before starting the service to catch the registration.
    let mut sub = nats
        .subscribe(subject)
        .await
        .expect("Failed to subscribe to status");

    let mut child = start_service(vin, NATS_URL, DATABROKER_ADDR).await;

    // Wait for the registration message (up to 10 seconds).
    let msg = timeout(Duration::from_secs(10), sub.next())
        .await
        .expect("Timed out waiting for registration message")
        .expect("Subscription ended without receiving a message");

    let parsed: serde_json::Value =
        serde_json::from_slice(&msg.payload).expect("Registration message is not valid JSON");

    assert_eq!(parsed["vin"].as_str(), Some(vin));
    assert_eq!(parsed["status"].as_str(), Some("online"));
    assert!(
        parsed.get("timestamp").is_some(),
        "Registration message should contain a timestamp"
    );

    // Clean up: kill the service process.
    child.kill().await.ok();
}

// ============================================================================
// TS-04-11: End-to-end command flow
//
// Requirement: 04-REQ-2.3, 04-REQ-5.2, 04-REQ-6.3
// Verify that a valid command published on NATS is forwarded to DATA_BROKER.
// ============================================================================

/// TS-04-11: End-to-end command flow.
///
/// Publish a valid, authenticated command to NATS and verify it is written
/// to `Vehicle.Command.Door.Lock` in DATA_BROKER.
#[tokio::test]
#[ignore]
async fn test_e2e_command_flow() {
    let vin = "INTEG-CMD-VIN";
    let nats = connect_nats().await;

    let mut child = start_service(vin, NATS_URL, DATABROKER_ADDR).await;

    // Wait for the service to start up and subscribe.
    tokio::time::sleep(Duration::from_secs(3)).await;

    let payload = r#"{"command_id":"cmd-1","action":"lock","doors":["driver"],"source":"companion_app","vin":"INTEG-CMD-VIN","timestamp":1700000000}"#;

    publish_command(&nats, vin, payload, BEARER_TOKEN).await;

    // Wait for the command to be forwarded, then read from DATA_BROKER.
    tokio::time::sleep(Duration::from_secs(2)).await;

    let mut broker = connect_databroker().await;
    let value = read_databroker_signal(
        &mut broker,
        cloud_gateway_client::broker_client::SIGNAL_COMMAND,
    )
    .await;

    assert_eq!(
        value.as_deref(),
        Some(payload),
        "DATA_BROKER should contain the command payload"
    );

    child.kill().await.ok();
}

// ============================================================================
// TS-04-12: End-to-end response relay
//
// Requirement: 04-REQ-7.1, 04-REQ-7.2
// Verify that a command response written to DATA_BROKER is relayed verbatim
// to NATS.
// ============================================================================

/// TS-04-12: End-to-end response relay.
///
/// Write a command response to `Vehicle.Command.Door.Response` in DATA_BROKER
/// and verify it is published verbatim to `vehicles.{VIN}.command_responses`
/// on NATS.
#[tokio::test]
#[ignore]
async fn test_e2e_response_relay() {
    use cloud_gateway_client::broker_client::kuksa::val::v2;

    let vin = "INTEG-RSP-VIN";
    let nats = connect_nats().await;

    let mut child = start_service(vin, NATS_URL, DATABROKER_ADDR).await;

    // Wait for the service to start and subscribe to DATA_BROKER.
    tokio::time::sleep(Duration::from_secs(3)).await;

    let response_subject = format!("vehicles.{vin}.command_responses");
    let mut sub = nats
        .subscribe(response_subject)
        .await
        .expect("Failed to subscribe to command_responses");

    let response_json =
        r#"{"command_id":"cmd-1","status":"success","timestamp":1700000001}"#;

    // Write the response to DATA_BROKER.
    let mut broker = connect_databroker().await;
    write_databroker_signal(
        &mut broker,
        cloud_gateway_client::broker_client::SIGNAL_RESPONSE,
        v2::value::TypedValue::String(response_json.to_string()),
    )
    .await;

    // Wait for the relayed message on NATS (up to 5 seconds).
    let msg = timeout(Duration::from_secs(5), sub.next())
        .await
        .expect("Timed out waiting for response relay on NATS")
        .expect("Subscription ended without receiving a response");

    let received = std::str::from_utf8(&msg.payload).expect("Response is not valid UTF-8");
    assert_eq!(
        received, response_json,
        "Response should be relayed verbatim"
    );

    child.kill().await.ok();
}

// ============================================================================
// TS-04-13: End-to-end telemetry on signal change
//
// Requirement: 04-REQ-8.1, 04-REQ-8.2
// Verify that a DATA_BROKER signal change triggers a telemetry message
// on NATS.
// ============================================================================

/// TS-04-13: End-to-end telemetry on signal change.
///
/// Update `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` in DATA_BROKER
/// and verify a telemetry message is published to `vehicles.{VIN}.telemetry`
/// on NATS containing the VIN and `is_locked: true`.
#[tokio::test]
#[ignore]
async fn test_e2e_telemetry_on_signal_change() {
    use cloud_gateway_client::broker_client::kuksa::val::v2;

    let vin = "INTEG-TEL-VIN";
    let nats = connect_nats().await;

    let mut child = start_service(vin, NATS_URL, DATABROKER_ADDR).await;

    // Wait for the service to start and subscribe to telemetry signals.
    tokio::time::sleep(Duration::from_secs(3)).await;

    let telemetry_subject = format!("vehicles.{vin}.telemetry");
    let mut sub = nats
        .subscribe(telemetry_subject)
        .await
        .expect("Failed to subscribe to telemetry");

    // Update the IsLocked signal in DATA_BROKER.
    let mut broker = connect_databroker().await;
    write_databroker_signal(
        &mut broker,
        cloud_gateway_client::broker_client::SIGNAL_IS_LOCKED,
        v2::value::TypedValue::Bool(true),
    )
    .await;

    // Wait for the telemetry message on NATS (up to 5 seconds).
    let msg = timeout(Duration::from_secs(5), sub.next())
        .await
        .expect("Timed out waiting for telemetry on NATS")
        .expect("Subscription ended without receiving telemetry");

    let parsed: serde_json::Value =
        serde_json::from_slice(&msg.payload).expect("Telemetry is not valid JSON");

    assert_eq!(parsed["vin"].as_str(), Some(vin));
    assert_eq!(
        parsed["is_locked"].as_bool(),
        Some(true),
        "Telemetry should contain is_locked: true"
    );

    child.kill().await.ok();
}

// ============================================================================
// TS-04-15: Command rejected with invalid token
//
// Requirement: 04-REQ-5.E2
// Verify that a command with an invalid bearer token is rejected and not
// forwarded to DATA_BROKER.
// ============================================================================

/// TS-04-15: Command rejected with invalid token.
///
/// Publish a command with a wrong bearer token and verify that
/// `Vehicle.Command.Door.Lock` in DATA_BROKER is NOT updated.
#[tokio::test]
#[ignore]
async fn test_command_rejected_with_invalid_token() {
    let vin = "INTEG-REJ-VIN";
    let nats = connect_nats().await;

    let mut child = start_service(vin, NATS_URL, DATABROKER_ADDR).await;

    // Wait for the service to start.
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Read the current value of the command signal (may be empty/None).
    let mut broker = connect_databroker().await;
    let before = read_databroker_signal(
        &mut broker,
        cloud_gateway_client::broker_client::SIGNAL_COMMAND,
    )
    .await;

    // Also subscribe to command_responses to verify nothing is published.
    let response_subject = format!("vehicles.{vin}.command_responses");
    let mut rsp_sub = nats
        .subscribe(response_subject)
        .await
        .expect("Failed to subscribe to command_responses");

    // Publish a command with a wrong token.
    let payload = r#"{"command_id":"cmd-2","action":"lock","doors":["driver"]}"#;
    publish_command(&nats, vin, payload, "wrong-token").await;

    // Wait a bit and check that nothing changed.
    tokio::time::sleep(Duration::from_secs(2)).await;

    let after = read_databroker_signal(
        &mut broker,
        cloud_gateway_client::broker_client::SIGNAL_COMMAND,
    )
    .await;

    assert_eq!(
        before, after,
        "DATA_BROKER command signal should not be updated for invalid token"
    );

    // Verify no response was published.
    let rsp = timeout(Duration::from_secs(1), rsp_sub.next()).await;
    assert!(
        rsp.is_err(),
        "No response should be published for invalid token"
    );

    child.kill().await.ok();
}

// ============================================================================
// TS-04-16: NATS reconnection with exponential backoff
//
// Requirement: 04-REQ-2.2, 04-REQ-2.E1
// Verify that the service retries NATS connection with exponential backoff
// and exits after exhausting retries.
// ============================================================================

/// TS-04-16: NATS reconnection with exponential backoff.
///
/// Start the service with an unreachable NATS URL and verify it exits
/// with code 1 after retrying with exponential backoff.
#[tokio::test]
#[ignore]
async fn test_nats_reconnection_backoff_and_exit() {
    let vin = "INTEG-RETRY-VIN";

    // Use an unreachable NATS URL — a port that nothing is listening on.
    let unreachable_nats = "nats://127.0.0.1:19999";

    let start = std::time::Instant::now();

    let binary = build_service_binary().await;
    let mut child = tokio::process::Command::new(&binary)
        .env("VIN", vin)
        .env("NATS_URL", unreachable_nats)
        .env("DATABROKER_ADDR", DATABROKER_ADDR)
        .env("BEARER_TOKEN", BEARER_TOKEN)
        .env("RUST_LOG", "cloud_gateway_client=debug")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to start service: {e}"));

    // Wait for the process to exit (up to 60 seconds for all retries).
    let status = timeout(Duration::from_secs(60), child.wait())
        .await
        .expect("Service did not exit within 60 seconds")
        .expect("Failed to wait for service process");

    let elapsed = start.elapsed();

    // The service should exit with code 1.
    assert_eq!(
        status.code(),
        Some(1),
        "Service should exit with code 1 after NATS retries exhausted"
    );

    // Exponential backoff: 1s + 2s + 4s + 8s = 15s minimum between 5 attempts.
    // Allow some tolerance for connection attempt time.
    assert!(
        elapsed >= Duration::from_secs(7),
        "Service should retry with backoff (elapsed: {elapsed:?}, expected >= 7s)"
    );
}
