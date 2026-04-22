//! Integration tests for cloud-gateway-client.
//!
//! These tests require NATS and DATA_BROKER (kuksa-databroker) containers
//! running. Start them with:
//!
//! ```sh
//! cd deployments && podman-compose up -d
//! ```
//!
//! Run (serially, since tests share DATA_BROKER signal state):
//!
//! ```sh
//! cd rhivos && cargo test -p cloud-gateway-client -- --ignored --test-threads=1
//! ```
//!
//! Test cases:
//! - TS-04-10: End-to-end command flow
//! - TS-04-11: End-to-end response relay
//! - TS-04-12: End-to-end telemetry on signal change
//! - TS-04-13: Self-registration on startup
//! - TS-04-14: Command rejected with invalid token
//! - TS-04-15: NATS reconnection with exponential backoff
//! - TS-04-SMOKE-1: Service starts with valid configuration
//! - TS-04-SMOKE-2: Service exits on missing VIN
//! - TS-04-SMOKE-3: Service publishes registration on startup

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use cloud_gateway_client::broker_client::kuksa::val::v1::{
    datapoint::Value, val_client::ValClient, DataEntry, Datapoint, EntryUpdate, Field, SetRequest,
    SubscribeEntry, SubscribeRequest, SubscribeResponse, View,
};
use futures::StreamExt;
use tokio::time::{sleep, timeout};
use tonic::transport::Channel;

const NATS_URL: &str = "nats://localhost:4222";
const BROKER_ADDR: &str = "http://localhost:55556";
const BEARER_TOKEN: &str = "demo-token";

const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";
const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// RAII guard that kills the cloud-gateway-client subprocess on drop.
///
/// Ensures the service process is cleaned up even if the test panics.
struct ServiceGuard(Child);

impl Drop for ServiceGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Start the cloud-gateway-client binary as a subprocess with the given VIN.
fn start_service(vin: &str) -> ServiceGuard {
    let child = Command::new(env!("CARGO_BIN_EXE_cloud-gateway-client"))
        .env("VIN", vin)
        .env("NATS_URL", NATS_URL)
        .env("DATABROKER_ADDR", BROKER_ADDR)
        .env("BEARER_TOKEN", BEARER_TOKEN)
        .env("RUST_LOG", "info")
        .spawn()
        .expect("failed to start cloud-gateway-client binary");
    ServiceGuard(child)
}

/// Start the cloud-gateway-client binary with captured stderr.
///
/// Returns the raw `Child` so the caller can read captured output after
/// killing or waiting on the process.
fn start_service_with_output(vin: &str) -> Child {
    Command::new(env!("CARGO_BIN_EXE_cloud-gateway-client"))
        .env("VIN", vin)
        .env("NATS_URL", NATS_URL)
        .env("DATABROKER_ADDR", BROKER_ADDR)
        .env("BEARER_TOKEN", BEARER_TOKEN)
        .env("RUST_LOG", "info")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start cloud-gateway-client binary")
}

/// Connect to the NATS server.
async fn connect_nats() -> async_nats::Client {
    async_nats::connect(NATS_URL)
        .await
        .expect("failed to connect to NATS")
}

/// Connect to the DATA_BROKER gRPC service.
async fn connect_broker() -> ValClient<Channel> {
    ValClient::connect(BROKER_ADDR)
        .await
        .expect("failed to connect to DATA_BROKER")
}

/// Wait for the service to publish its registration message on NATS.
///
/// Subscribes to `vehicles.{VIN}.status` and blocks until a message arrives
/// or times out after 10 seconds.
async fn wait_for_service_ready(nats: &async_nats::Client, vin: &str) {
    let subject = format!("vehicles.{vin}.status");
    let mut sub = nats
        .subscribe(subject)
        .await
        .expect("failed to subscribe to status");
    timeout(Duration::from_secs(10), sub.next())
        .await
        .expect("service did not publish registration within 10 seconds")
        .expect("status subscription yielded None");
}

/// Set a string-type signal in DATA_BROKER via gRPC SetRequest.
async fn set_signal_string(broker: &mut ValClient<Channel>, path: &str, value: &str) {
    let request = tonic::Request::new(SetRequest {
        updates: vec![EntryUpdate {
            entry: Some(DataEntry {
                path: path.to_string(),
                value: Some(Datapoint {
                    timestamp: 0,
                    value: Some(Value::StringValue(value.to_string())),
                }),
                actuator_target: None,
            }),
            fields: vec![Field::Value as i32],
        }],
    });
    broker
        .set(request)
        .await
        .unwrap_or_else(|e| panic!("failed to set {path}: {e}"));
}

/// Set a boolean-type signal in DATA_BROKER via gRPC SetRequest.
async fn set_signal_bool(broker: &mut ValClient<Channel>, path: &str, value: bool) {
    let request = tonic::Request::new(SetRequest {
        updates: vec![EntryUpdate {
            entry: Some(DataEntry {
                path: path.to_string(),
                value: Some(Datapoint {
                    timestamp: 0,
                    value: Some(Value::BoolValue(value)),
                }),
                actuator_target: None,
            }),
            fields: vec![Field::Value as i32],
        }],
    });
    broker
        .set(request)
        .await
        .unwrap_or_else(|e| panic!("failed to set {path}: {e}"));
}

/// Subscribe to a DATA_BROKER signal via gRPC Subscribe and return the stream.
async fn subscribe_signal(
    broker: &mut ValClient<Channel>,
    path: &str,
) -> tonic::Streaming<SubscribeResponse> {
    let request = tonic::Request::new(SubscribeRequest {
        entries: vec![SubscribeEntry {
            path: path.to_string(),
            view: View::CurrentValue as i32,
            fields: vec![Field::Value as i32],
        }],
    });
    broker
        .subscribe(request)
        .await
        .unwrap_or_else(|e| panic!("failed to subscribe to {path}: {e}"))
        .into_inner()
}

// ---------------------------------------------------------------------------
// TS-04-10: End-to-end command flow
// ---------------------------------------------------------------------------

/// TS-04-10: End-to-end command flow.
///
/// Validates: [04-REQ-5.2], [04-REQ-6.3], [04-REQ-2.3]
///
/// GIVEN CLOUD_GATEWAY_CLIENT is running with VIN="E2E-CMD-VIN"
/// WHEN a NATS message is published to "vehicles.E2E-CMD-VIN.commands"
///   WITH header "Authorization" = "Bearer demo-token"
///   WITH valid command payload
/// THEN within 2 seconds, Vehicle.Command.Door.Lock in DATA_BROKER
///   contains the command payload.
#[tokio::test]
#[ignore]
async fn test_e2e_command_flow() {
    let vin = "E2E-CMD-VIN";
    let nats = connect_nats().await;
    let mut broker = connect_broker().await;

    // Subscribe to the command signal BEFORE starting the service so we
    // don't miss the write event.
    let mut signal_stream = subscribe_signal(&mut broker, SIGNAL_COMMAND).await;

    // Start service and wait for readiness.
    let _guard = start_service(vin);
    wait_for_service_ready(&nats, vin).await;

    // Allow initial DATA_BROKER subscription snapshots to settle.
    sleep(Duration::from_millis(300)).await;

    // Publish a valid command to NATS with correct bearer token.
    let payload = r#"{"command_id":"cmd-1","action":"lock","doors":["driver"],"source":"companion_app","vin":"E2E-CMD-VIN","timestamp":1700000000}"#;
    let mut headers = async_nats::HeaderMap::new();
    headers.insert("Authorization", format!("Bearer {BEARER_TOKEN}"));
    nats.publish_with_headers(
        format!("vehicles.{vin}.commands"),
        headers,
        payload.to_owned().into(),
    )
    .await
    .expect("failed to publish command to NATS");
    nats.flush().await.expect("failed to flush NATS");

    // Wait for the exact payload to appear in DATA_BROKER within 2 seconds.
    // We search through subscription updates for our specific payload to
    // handle any leftover values from previous signal states.
    let found = timeout(Duration::from_secs(2), async {
        while let Some(Ok(response)) = signal_stream.next().await {
            for update in response.updates {
                if let Some(dp) = update.value {
                    if let Some(Value::StringValue(ref s)) = dp.value {
                        if s == payload {
                            return true;
                        }
                    }
                }
            }
        }
        false
    })
    .await;

    assert!(
        found.unwrap_or(false),
        "command payload was not written to {SIGNAL_COMMAND} within 2 seconds"
    );
}

// ---------------------------------------------------------------------------
// TS-04-11: End-to-end response relay
// ---------------------------------------------------------------------------

/// TS-04-11: End-to-end response relay.
///
/// Validates: [04-REQ-7.1], [04-REQ-7.2]
///
/// GIVEN CLOUD_GATEWAY_CLIENT is running with VIN="E2E-RSP-VIN"
/// GIVEN NATS subscriber is listening on "vehicles.E2E-RSP-VIN.command_responses"
/// WHEN Vehicle.Command.Door.Response is set in DATA_BROKER
/// THEN within 2 seconds, the NATS subscriber receives the response JSON verbatim.
#[tokio::test]
#[ignore]
async fn test_e2e_response_relay() {
    let vin = "E2E-RSP-VIN";
    let nats = connect_nats().await;
    let mut broker = connect_broker().await;

    // Subscribe to command responses on NATS BEFORE starting the service.
    let mut response_sub = nats
        .subscribe(format!("vehicles.{vin}.command_responses"))
        .await
        .expect("failed to subscribe to command responses");

    // Start service and wait for readiness.
    let _guard = start_service(vin);
    wait_for_service_ready(&nats, vin).await;

    // Give the service time to set up its DATA_BROKER subscriptions.
    sleep(Duration::from_millis(500)).await;

    // Write a response to DATA_BROKER.
    let response_json =
        r#"{"command_id":"cmd-1","status":"success","timestamp":1700000001}"#;
    set_signal_string(&mut broker, SIGNAL_RESPONSE, response_json).await;

    // Wait for the response to appear on NATS within 2 seconds.
    // Search through messages for the exact match (handles any leftover
    // values from previous DATA_BROKER state).
    let found = timeout(Duration::from_secs(2), async {
        while let Some(msg) = response_sub.next().await {
            let received = std::str::from_utf8(&msg.payload).unwrap_or_default();
            if received == response_json {
                return true;
            }
        }
        false
    })
    .await;

    assert!(
        found.unwrap_or(false),
        "response was not relayed verbatim to NATS within 2 seconds"
    );
}

// ---------------------------------------------------------------------------
// TS-04-12: End-to-end telemetry on signal change
// ---------------------------------------------------------------------------

/// TS-04-12: End-to-end telemetry on signal change.
///
/// Validates: [04-REQ-8.1], [04-REQ-8.2]
///
/// GIVEN CLOUD_GATEWAY_CLIENT is running with VIN="E2E-TEL-VIN"
/// GIVEN NATS subscriber is listening on "vehicles.E2E-TEL-VIN.telemetry"
/// WHEN Vehicle.Cabin.Door.Row1.DriverSide.IsLocked is set to true in DATA_BROKER
/// THEN within 2 seconds, the NATS subscriber receives a telemetry JSON
///   AND the JSON contains "vin":"E2E-TEL-VIN"
///   AND the JSON contains "is_locked":true.
#[tokio::test]
#[ignore]
async fn test_e2e_telemetry_signal_change() {
    let vin = "E2E-TEL-VIN";
    let nats = connect_nats().await;
    let mut broker = connect_broker().await;

    // Subscribe to telemetry on NATS BEFORE starting the service.
    let mut telemetry_sub = nats
        .subscribe(format!("vehicles.{vin}.telemetry"))
        .await
        .expect("failed to subscribe to telemetry");

    // Start service and wait for readiness.
    let _guard = start_service(vin);
    wait_for_service_ready(&nats, vin).await;

    // Give the service time to set up its DATA_BROKER subscriptions.
    sleep(Duration::from_millis(500)).await;

    // Set IsLocked to true in DATA_BROKER.
    set_signal_bool(&mut broker, SIGNAL_IS_LOCKED, true).await;

    // Wait for a telemetry message containing is_locked:true within 2 seconds.
    // Search through messages because the service may publish initial telemetry
    // from DATA_BROKER subscription snapshots before our explicit set.
    let found = timeout(Duration::from_secs(2), async {
        while let Some(msg) = telemetry_sub.next().await {
            let payload = std::str::from_utf8(&msg.payload).unwrap_or_default();
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(payload) {
                if parsed["vin"] == vin && parsed["is_locked"] == true {
                    return true;
                }
            }
        }
        false
    })
    .await;

    assert!(
        found.unwrap_or(false),
        "telemetry with vin:{vin} and is_locked:true not received within 2 seconds"
    );
}

// ---------------------------------------------------------------------------
// TS-04-13: Self-registration on startup
// ---------------------------------------------------------------------------

/// TS-04-13: Self-registration on startup.
///
/// Validates: [04-REQ-4.1], [04-REQ-4.2]
///
/// GIVEN NATS subscriber is listening on "vehicles.E2E-REG-VIN.status"
/// WHEN CLOUD_GATEWAY_CLIENT is started with VIN="E2E-REG-VIN"
/// THEN within 5 seconds, the NATS subscriber receives a registration message
///   AND the JSON contains "vin":"E2E-REG-VIN"
///   AND the JSON contains "status":"online"
///   AND the JSON contains "timestamp".
#[tokio::test]
#[ignore]
async fn test_e2e_self_registration() {
    let vin = "E2E-REG-VIN";
    let nats = connect_nats().await;

    // Subscribe to registration BEFORE starting the service.
    let mut status_sub = nats
        .subscribe(format!("vehicles.{vin}.status"))
        .await
        .expect("failed to subscribe to status");

    // Start service (do NOT call wait_for_service_ready since the
    // registration message IS what we're testing).
    let _guard = start_service(vin);

    // Wait for the registration message within 5 seconds.
    let msg = timeout(Duration::from_secs(5), status_sub.next())
        .await
        .expect("did not receive registration message within 5 seconds")
        .expect("status subscription yielded None");

    let received = std::str::from_utf8(&msg.payload)
        .expect("registration payload is not valid UTF-8");
    let parsed: serde_json::Value =
        serde_json::from_str(received).expect("registration payload is not valid JSON");

    assert_eq!(
        parsed["vin"], vin,
        "registration should contain correct VIN"
    );
    assert_eq!(
        parsed["status"], "online",
        "registration should have status:online"
    );
    assert!(
        parsed.get("timestamp").is_some(),
        "registration should contain a timestamp"
    );
}

// ---------------------------------------------------------------------------
// TS-04-14: Command rejected with invalid token
// ---------------------------------------------------------------------------

/// TS-04-14: Command rejected with invalid token.
///
/// Validates: [04-REQ-5.E2]
///
/// GIVEN CLOUD_GATEWAY_CLIENT is running with VIN="E2E-REJ-VIN"
/// WHEN a NATS message is published to "vehicles.E2E-REJ-VIN.commands"
///   WITH header "Authorization" = "Bearer wrong-token"
///   WITH valid command payload
/// THEN Vehicle.Command.Door.Lock in DATA_BROKER is NOT updated
///   AND no message is published to "vehicles.E2E-REJ-VIN.command_responses".
#[tokio::test]
#[ignore]
async fn test_e2e_command_rejected_invalid_token() {
    let vin = "E2E-REJ-VIN";
    let nats = connect_nats().await;
    let mut broker = connect_broker().await;

    // Subscribe to command responses on NATS.
    let mut response_sub = nats
        .subscribe(format!("vehicles.{vin}.command_responses"))
        .await
        .expect("failed to subscribe to command responses");

    // Start service and wait for readiness.
    let _guard = start_service(vin);
    wait_for_service_ready(&nats, vin).await;

    // Give the service time to set up all subscriptions.
    sleep(Duration::from_millis(500)).await;

    // Subscribe to command signal in DATA_BROKER and drain any initial snapshot.
    let mut signal_stream = subscribe_signal(&mut broker, SIGNAL_COMMAND).await;
    sleep(Duration::from_millis(300)).await;
    while timeout(Duration::from_millis(100), signal_stream.next())
        .await
        .is_ok()
    {}

    // Publish command with WRONG bearer token.
    let payload = r#"{"command_id":"cmd-rej","action":"lock","doors":["driver"]}"#;
    let mut headers = async_nats::HeaderMap::new();
    headers.insert("Authorization", "Bearer wrong-token");
    nats.publish_with_headers(
        format!("vehicles.{vin}.commands"),
        headers,
        payload.to_owned().into(),
    )
    .await
    .expect("failed to publish command");
    nats.flush().await.expect("failed to flush NATS");

    // Verify no update to Vehicle.Command.Door.Lock within 2 seconds.
    let signal_result = timeout(Duration::from_secs(2), signal_stream.next()).await;
    assert!(
        signal_result.is_err(),
        "{SIGNAL_COMMAND} should NOT be updated when bearer token is invalid"
    );

    // Verify no command response was published to NATS.
    let response_result = timeout(Duration::from_secs(1), response_sub.next()).await;
    assert!(
        response_result.is_err(),
        "no command response should be published when bearer token is invalid"
    );
}

// ---------------------------------------------------------------------------
// TS-04-SMOKE-1: Service starts with valid configuration
// ---------------------------------------------------------------------------

/// TS-04-SMOKE-1: Service starts with valid configuration.
///
/// Validates: [04-REQ-2.1], [04-REQ-3.1]
///
/// GIVEN NATS container is running on localhost:4222
/// GIVEN DATA_BROKER container is running on localhost:55556
/// GIVEN env VIN="SMOKE-VIN"
/// WHEN CLOUD_GATEWAY_CLIENT binary is executed
/// THEN the process starts without error
///   AND logs contain "Connected to NATS"
///   AND logs contain "Connected to DATA_BROKER"
#[tokio::test]
#[ignore]
async fn test_smoke_service_starts_with_valid_config() {
    let vin = "SMOKE-START-VIN";
    let nats = connect_nats().await;

    // Subscribe to status BEFORE starting service to catch the registration
    // message, which proves the service completed its full startup sequence.
    let mut status_sub = nats
        .subscribe(format!("vehicles.{vin}.status"))
        .await
        .expect("failed to subscribe to status");

    // Start service with captured stderr.
    let mut child = start_service_with_output(vin);

    // Wait for the registration message — this proves the service connected
    // to both NATS and DATA_BROKER and completed the startup sequence.
    timeout(Duration::from_secs(10), status_sub.next())
        .await
        .expect("service did not publish registration within 10 seconds")
        .expect("status subscription yielded None");

    // Allow final log messages to flush.
    sleep(Duration::from_millis(200)).await;

    // Kill the service and collect output.
    child.kill().expect("failed to kill service");
    let output = child.wait_with_output().expect("failed to get output");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Connected to NATS"),
        "logs should contain 'Connected to NATS', got:\n{stderr}"
    );
    assert!(
        stderr.contains("Connected to DATA_BROKER"),
        "logs should contain 'Connected to DATA_BROKER', got:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// TS-04-SMOKE-2: Service exits on missing VIN
// ---------------------------------------------------------------------------

/// TS-04-SMOKE-2: Service exits on missing VIN.
///
/// Validates: [04-REQ-1.E1]
///
/// GIVEN env VIN is not set
/// WHEN CLOUD_GATEWAY_CLIENT binary is executed
/// THEN the process exits with code 1
///   AND stderr contains "VIN"
#[tokio::test]
#[ignore]
async fn test_smoke_exits_on_missing_vin() {
    let output = Command::new(env!("CARGO_BIN_EXE_cloud-gateway-client"))
        .env_remove("VIN")
        .env("RUST_LOG", "error")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to run service");

    assert_eq!(
        output.status.code(),
        Some(1),
        "service should exit with code 1 when VIN is missing"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("VIN"),
        "stderr should mention VIN, got:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// TS-04-SMOKE-3: Service publishes registration on startup
// ---------------------------------------------------------------------------

/// TS-04-SMOKE-3: Service publishes registration on startup.
///
/// Validates: [04-REQ-4.1]
///
/// GIVEN NATS container is running on localhost:4222
/// GIVEN DATA_BROKER container is running on localhost:55556
/// GIVEN NATS subscriber is listening on "vehicles.SMOKE-VIN.status"
/// GIVEN env VIN="SMOKE-VIN"
/// WHEN CLOUD_GATEWAY_CLIENT binary is executed
/// THEN within 5 seconds, a registration message is received on
///   "vehicles.SMOKE-VIN.status"
#[tokio::test]
#[ignore]
async fn test_smoke_registration_on_startup() {
    let vin = "SMOKE-REG-VIN";
    let nats = connect_nats().await;

    // Subscribe to registration BEFORE starting the service.
    let mut status_sub = nats
        .subscribe(format!("vehicles.{vin}.status"))
        .await
        .expect("failed to subscribe to status");

    let _guard = start_service(vin);

    // Wait for the registration message within 5 seconds.
    let msg = timeout(Duration::from_secs(5), status_sub.next())
        .await
        .expect("did not receive registration message within 5 seconds")
        .expect("status subscription yielded None");

    let received = std::str::from_utf8(&msg.payload)
        .expect("registration payload is not valid UTF-8");
    let parsed: serde_json::Value =
        serde_json::from_str(received).expect("registration payload is not valid JSON");

    assert_eq!(
        parsed["vin"], vin,
        "registration should contain correct VIN"
    );
    assert_eq!(
        parsed["status"], "online",
        "registration should have status:online"
    );
}

// ---------------------------------------------------------------------------
// TS-04-15: NATS reconnection with exponential backoff
// ---------------------------------------------------------------------------

/// TS-04-15: NATS reconnection with exponential backoff.
///
/// Validates: [04-REQ-2.2], [04-REQ-2.E1]
///
/// GIVEN NATS server is not running (unreachable URL)
/// WHEN CLOUD_GATEWAY_CLIENT is started
/// THEN the service attempts to connect with exponential backoff
///   AND after 5 failed attempts, the service exits with code 1
///
/// Note: The implementation uses delays of 1s, 2s, 4s, 8s between 5
/// connection attempts, totaling ~15 seconds. See
/// `docs/errata/04_nats_reconnection_delays.md` for the specification
/// inconsistency.
#[tokio::test]
#[ignore]
async fn test_nats_reconnection_backoff() {
    // Use a port where no NATS server is listening.
    let unreachable_nats = "nats://127.0.0.1:19876";

    let start = std::time::Instant::now();

    let output = Command::new(env!("CARGO_BIN_EXE_cloud-gateway-client"))
        .env("VIN", "BACKOFF-VIN")
        .env("NATS_URL", unreachable_nats)
        .env("DATABROKER_ADDR", BROKER_ADDR)
        .env("RUST_LOG", "info")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to run service");

    let elapsed = start.elapsed();

    // Service should exit with code 1 after all retries are exhausted.
    assert_eq!(
        output.status.code(),
        Some(1),
        "service should exit with code 1 after NATS retries exhausted"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify logs indicate retries occurred.
    assert!(
        stderr.contains("NATS connection failed")
            || stderr.contains("retries exhausted")
            || stderr.contains("RetriesExhausted"),
        "stderr should mention NATS connection failure, got:\n{stderr}"
    );

    // Verify exponential backoff timing.
    // Total delays: 1s + 2s + 4s + 8s = 15s, plus connection attempt time.
    // Allow tolerance: at least 12 seconds (delays may be slightly shorter
    // due to fast connection-refused errors), at most 45 seconds.
    assert!(
        elapsed.as_secs() >= 12,
        "should take at least 12 seconds for backoff delays, took {elapsed:?}"
    );
    assert!(
        elapsed.as_secs() <= 45,
        "should take at most 45 seconds, took {elapsed:?}"
    );
}

// ---------------------------------------------------------------------------
// TS-04-P6 (partial): Startup determinism — broker unreachable
// ---------------------------------------------------------------------------

/// Startup determinism: DATA_BROKER unreachable exits before registration.
///
/// Validates: [04-REQ-3.E1], [04-REQ-9.2] (partial)
///
/// GIVEN NATS is running but DATA_BROKER is unreachable
/// WHEN CLOUD_GATEWAY_CLIENT is started
/// THEN the service exits with code 1
///   AND no registration message is published to NATS
///
/// This verifies that a failure at step 3 (DATA_BROKER connection) prevents
/// step 4 (self-registration) and beyond from executing.
#[tokio::test]
#[ignore]
async fn test_startup_exits_on_unreachable_broker() {
    let vin = "BROKER-FAIL-VIN";
    let nats = connect_nats().await;

    // Subscribe to status to verify NO registration is published.
    let mut status_sub = nats
        .subscribe(format!("vehicles.{vin}.status"))
        .await
        .expect("failed to subscribe to status");

    // Start service with an unreachable DATA_BROKER address.
    let output = Command::new(env!("CARGO_BIN_EXE_cloud-gateway-client"))
        .env("VIN", vin)
        .env("NATS_URL", NATS_URL)
        .env("DATABROKER_ADDR", "http://127.0.0.1:19877")
        .env("RUST_LOG", "info")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to run service");

    // Service should exit with code 1.
    assert_eq!(
        output.status.code(),
        Some(1),
        "service should exit with code 1 when DATA_BROKER is unreachable"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("DATA_BROKER") || stderr.contains("broker"),
        "stderr should mention DATA_BROKER failure, got:\n{stderr}"
    );

    // Verify no registration was published (startup determinism: step 3
    // failure prevents step 4).
    let reg_result = timeout(Duration::from_secs(1), status_sub.next()).await;
    assert!(
        reg_result.is_err(),
        "no registration should be published when DATA_BROKER connection fails"
    );
}
