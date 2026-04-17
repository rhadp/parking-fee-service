//! Integration tests for CLOUD_GATEWAY_CLIENT.
//!
//! These tests require running NATS and DATA_BROKER (Kuksa) containers:
//!
//! ```sh
//! mkdir -p /tmp/kuksa
//! cd deployments && podman compose up -d
//! ```
//!
//! Run all integration tests with:
//!
//! ```sh
//! cargo test -p cloud-gateway-client -- --ignored
//! ```
//!
//! Tests that cannot reach NATS or DATA_BROKER will print a SKIP message and
//! return without failing, so `cargo test -- --ignored` is always safe to run.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use futures::StreamExt;

// Re-include the generated proto types so tests can interact with DATA_BROKER
// directly (write signals, read signal values) without going through the service.
// Uses the same kuksa.val.v2 proto that the main implementation uses.
#[allow(clippy::enum_variant_names)]
mod kuksa_v2 {
    tonic::include_proto!("kuksa.val.v2");
}

use kuksa_v2::val_client::ValClient;
use kuksa_v2::{Datapoint, GetValueRequest, PublishValueRequest, SignalId, Value};
use kuksa_v2::signal_id::Signal;
use kuksa_v2::value::TypedValue;
use tonic::transport::{Channel, Endpoint};

// ── Constants ─────────────────────────────────────────────────────────────────

const NATS_URL: &str = "nats://localhost:4222";
const BROKER_ADDR: &str = "http://localhost:55556";
const BEARER_TOKEN: &str = "demo-token";

const SIGNAL_COMMAND_LOCK: &str = "Vehicle.Command.Door.Lock";
const SIGNAL_COMMAND_RESPONSE: &str = "Vehicle.Command.Door.Response";
const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";

// ── Infrastructure helpers ────────────────────────────────────────────────────

/// Return `true` if NATS is reachable on the default port.
async fn nats_available() -> bool {
    tokio::time::timeout(Duration::from_secs(2), async_nats::connect(NATS_URL))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}

/// Return `true` if the DATA_BROKER gRPC endpoint is reachable.
async fn broker_available() -> bool {
    let endpoint = match Endpoint::from_shared(BROKER_ADDR.to_owned()) {
        Ok(ep) => ep.connect_timeout(Duration::from_secs(2)),
        Err(_) => return false,
    };
    tokio::time::timeout(Duration::from_secs(3), endpoint.connect())
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}

/// RAII guard: kills and reaps the child process on drop.
struct ServiceGuard(Child);

impl Drop for ServiceGuard {
    fn drop(&mut self) {
        self.0.kill().ok();
        self.0.wait().ok();
    }
}

/// Spawn the `cloud-gateway-client` binary with the given VIN and default URLs.
///
/// Returns a `ServiceGuard` that kills the process when it goes out of scope.
fn spawn_service(vin: &str) -> ServiceGuard {
    ServiceGuard(
        Command::new(env!("CARGO_BIN_EXE_cloud-gateway-client"))
            .env("VIN", vin)
            .env("NATS_URL", NATS_URL)
            .env("DATABROKER_ADDR", BROKER_ADDR)
            .env("BEARER_TOKEN", BEARER_TOKEN)
            .env("RUST_LOG", "warn")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to spawn cloud-gateway-client binary"),
    )
}

/// Wait for the next message on `subscriber`, with a `timeout_secs` deadline.
///
/// Returns `None` if no message arrives within the timeout.
async fn recv_with_timeout(
    subscriber: &mut async_nats::Subscriber,
    timeout_secs: u64,
) -> Option<async_nats::Message> {
    tokio::time::timeout(Duration::from_secs(timeout_secs), subscriber.next())
        .await
        .ok()
        .flatten()
}

/// Open a fresh gRPC connection to DATA_BROKER for test helper operations.
async fn raw_broker() -> ValClient<Channel> {
    ValClient::connect(BROKER_ADDR)
        .await
        .expect("Test helper: failed to connect to DATA_BROKER")
}

/// Read the current string value of a VSS signal from DATA_BROKER.
///
/// Returns `None` if the signal was never set or carries a non-string value.
async fn read_signal_string(path: &str) -> Option<String> {
    let mut client = raw_broker().await;
    let resp = client
        .get_value(GetValueRequest {
            signal_id: Some(SignalId {
                signal: Some(Signal::Path(path.to_owned())),
            }),
        })
        .await
        .ok()?
        .into_inner();

    if let Some(dp) = resp.data_point {
        if let Some(Value { typed_value: Some(TypedValue::String(s)) }) = dp.value {
            return Some(s);
        }
    }
    None
}

/// Write a string value to a VSS signal in DATA_BROKER using PublishValue.
async fn write_actuator_string(path: &str, value: &str) {
    let mut client = raw_broker().await;
    client
        .publish_value(PublishValueRequest {
            signal_id: Some(SignalId {
                signal: Some(Signal::Path(path.to_owned())),
            }),
            data_point: Some(Datapoint {
                value: Some(Value {
                    typed_value: Some(TypedValue::String(value.to_owned())),
                }),
            }),
        })
        .await
        .expect("Test helper: failed to publish string signal to DATA_BROKER");
}

/// Publish a string value to a VSS sensor signal in DATA_BROKER using PublishValue.
async fn publish_sensor_string(path: &str, value: &str) {
    let mut client = raw_broker().await;
    client
        .publish_value(PublishValueRequest {
            signal_id: Some(SignalId {
                signal: Some(Signal::Path(path.to_owned())),
            }),
            data_point: Some(Datapoint {
                value: Some(Value {
                    typed_value: Some(TypedValue::String(value.to_owned())),
                }),
            }),
        })
        .await
        .expect("Test helper: failed to publish string sensor value to DATA_BROKER");
}

/// Publish a boolean value to a VSS signal in DATA_BROKER using PublishValue.
async fn write_actuator_bool(path: &str, value: bool) {
    let mut client = raw_broker().await;
    client
        .publish_value(PublishValueRequest {
            signal_id: Some(SignalId {
                signal: Some(Signal::Path(path.to_owned())),
            }),
            data_point: Some(Datapoint {
                value: Some(Value {
                    typed_value: Some(TypedValue::Bool(value)),
                }),
            }),
        })
        .await
        .expect("Test helper: failed to publish bool signal to DATA_BROKER");
}

/// Wait for the service registration message on `vehicles.{vin}.status`.
///
/// Subscribes to the status subject and waits up to `timeout_secs` seconds.
/// Returns the parsed registration JSON, or panics with a clear message.
///
/// **Call this *before* spawning the service** so the subscription is in place
/// before the registration message is published.
async fn subscribe_for_registration(
    nats: &async_nats::Client,
    vin: &str,
) -> async_nats::Subscriber {
    let subject = format!("vehicles.{}.status", vin);
    nats.subscribe(subject)
        .await
        .expect("Subscribe to status subject")
}

// ── Integration Tests ─────────────────────────────────────────────────────────

/// TS-04-10: End-to-end command flow.
///
/// Validates: [04-REQ-2.3], [04-REQ-5.2], [04-REQ-6.3]
///
/// Publishes a valid NATS command with the correct bearer token and verifies
/// that `Vehicle.Command.Door.Lock` in DATA_BROKER is updated with the command
/// payload within 2 seconds.
#[tokio::test]
#[ignore]
async fn ts_04_10_end_to_end_command_flow() {
    if !nats_available().await {
        eprintln!("SKIP ts_04_10: NATS not reachable at {NATS_URL}");
        return;
    }
    if !broker_available().await {
        eprintln!("SKIP ts_04_10: DATA_BROKER not reachable at {BROKER_ADDR}");
        return;
    }

    let vin = "E2E-CMD-VIN";
    let nats = async_nats::connect(NATS_URL)
        .await
        .expect("Connect to NATS");

    // Subscribe to registration BEFORE spawning so we don't miss it.
    let mut reg_sub = subscribe_for_registration(&nats, vin).await;
    let _guard = spawn_service(vin);

    // Wait for the service to announce itself (proves it is past startup step 4).
    recv_with_timeout(&mut reg_sub, 10)
        .await
        .expect("ts_04_10: service should publish registration within 10 seconds");

    // Give the service a moment to complete DATA_BROKER subscriptions.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Publish a valid command with the correct bearer token.
    let command_payload = r#"{"command_id":"cmd-e2e-10","action":"lock","doors":["driver"],"source":"test","vin":"E2E-CMD-VIN","timestamp":1700000000}"#;
    let command_subject = format!("vehicles.{}.commands", vin);

    let mut headers = async_nats::HeaderMap::new();
    headers.insert("Authorization", format!("Bearer {BEARER_TOKEN}").as_str());

    nats.publish_with_headers(
        command_subject,
        headers,
        command_payload.as_bytes().to_vec().into(),
    )
    .await
    .expect("Publish command to NATS");

    // Poll DATA_BROKER for up to 2 seconds to verify the command was written.
    let mut received = false;
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if let Some(val) = read_signal_string(SIGNAL_COMMAND_LOCK).await {
            if val.contains("cmd-e2e-10") {
                received = true;
                break;
            }
        }
    }

    assert!(
        received,
        "ts_04_10: Vehicle.Command.Door.Lock must contain the command payload within 2 seconds"
    );
}

/// TS-04-11: End-to-end response relay.
///
/// Validates: [04-REQ-7.1], [04-REQ-7.2]
///
/// Writes a command response JSON string to `Vehicle.Command.Door.Response` in
/// DATA_BROKER and verifies that the service relays it verbatim to
/// `vehicles.{VIN}.command_responses` on NATS within 5 seconds.
#[tokio::test]
#[ignore]
async fn ts_04_11_end_to_end_response_relay() {
    if !nats_available().await {
        eprintln!("SKIP ts_04_11: NATS not reachable at {NATS_URL}");
        return;
    }
    if !broker_available().await {
        eprintln!("SKIP ts_04_11: DATA_BROKER not reachable at {BROKER_ADDR}");
        return;
    }

    let vin = "E2E-RSP-VIN";
    let nats = async_nats::connect(NATS_URL)
        .await
        .expect("Connect to NATS");

    // Subscribe to registration and command_responses before spawning.
    let mut reg_sub = subscribe_for_registration(&nats, vin).await;
    let response_subject = format!("vehicles.{}.command_responses", vin);
    let mut resp_sub = nats
        .subscribe(response_subject)
        .await
        .expect("Subscribe to command_responses");

    let _guard = spawn_service(vin);

    // Wait for service readiness.
    recv_with_timeout(&mut reg_sub, 10)
        .await
        .expect("ts_04_11: service should publish registration within 10 seconds");

    // Give the service a moment to complete DATA_BROKER subscriptions.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Write a command response to DATA_BROKER (sensor signal → PublishValue).
    let response_json =
        r#"{"command_id":"cmd-relay-11","status":"success","timestamp":1700000001}"#;
    publish_sensor_string(SIGNAL_COMMAND_RESPONSE, response_json).await;

    // Wait for the service to relay the response to NATS.
    let msg = recv_with_timeout(&mut resp_sub, 5)
        .await
        .expect("ts_04_11: service should relay command response within 5 seconds");

    let payload =
        std::str::from_utf8(&msg.payload).expect("ts_04_11: relay payload must be valid UTF-8");

    assert_eq!(
        payload, response_json,
        "ts_04_11: command response must be relayed verbatim"
    );
}

/// TS-04-12: End-to-end telemetry on signal change.
///
/// Validates: [04-REQ-8.1], [04-REQ-8.2]
///
/// Sets `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` to `true` in DATA_BROKER
/// and verifies that the service publishes a telemetry message to
/// `vehicles.{VIN}.telemetry` on NATS within 5 seconds, containing
/// `"vin":"E2E-TLM-VIN"` and `"is_locked":true`.
#[tokio::test]
#[ignore]
async fn ts_04_12_end_to_end_telemetry_on_signal_change() {
    if !nats_available().await {
        eprintln!("SKIP ts_04_12: NATS not reachable at {NATS_URL}");
        return;
    }
    if !broker_available().await {
        eprintln!("SKIP ts_04_12: DATA_BROKER not reachable at {BROKER_ADDR}");
        return;
    }

    let vin = "E2E-TLM-VIN";
    let nats = async_nats::connect(NATS_URL)
        .await
        .expect("Connect to NATS");

    // Subscribe to registration and telemetry before spawning.
    let mut reg_sub = subscribe_for_registration(&nats, vin).await;
    let telemetry_subject = format!("vehicles.{}.telemetry", vin);
    let mut telem_sub = nats
        .subscribe(telemetry_subject)
        .await
        .expect("Subscribe to telemetry");

    let _guard = spawn_service(vin);

    // Wait for service readiness.
    recv_with_timeout(&mut reg_sub, 10)
        .await
        .expect("ts_04_12: service should publish registration within 10 seconds");

    // Give the service time to set up DATA_BROKER subscriptions.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Set IsLocked to true in DATA_BROKER — this triggers a telemetry publish.
    // IsLocked is an actuator signal in the DATA_BROKER, so use Actuate.
    write_actuator_bool(SIGNAL_IS_LOCKED, true).await;

    // Wait for the telemetry message on NATS.
    let msg = recv_with_timeout(&mut telem_sub, 5)
        .await
        .expect("ts_04_12: service should publish telemetry within 5 seconds");

    let payload =
        std::str::from_utf8(&msg.payload).expect("ts_04_12: telemetry payload must be valid UTF-8");
    let json: serde_json::Value =
        serde_json::from_str(payload).expect("ts_04_12: telemetry must be valid JSON");

    assert_eq!(
        json["vin"], vin,
        "ts_04_12: telemetry must contain the correct VIN"
    );
    assert_eq!(
        json["is_locked"], true,
        "ts_04_12: telemetry must contain is_locked:true"
    );
    assert!(
        json["timestamp"].is_number(),
        "ts_04_12: telemetry must contain a numeric timestamp"
    );
}

/// TS-04-13: Self-registration on startup.
///
/// Validates: [04-REQ-4.1], [04-REQ-4.2]
///
/// Subscribes to `vehicles.REG-VIN.status` before starting the service, then
/// verifies that within 5 seconds the service publishes a registration message
/// with `"vin":"REG-VIN"` and `"status":"online"`.
#[tokio::test]
#[ignore]
async fn ts_04_13_self_registration_on_startup() {
    if !nats_available().await {
        eprintln!("SKIP ts_04_13: NATS not reachable at {NATS_URL}");
        return;
    }
    if !broker_available().await {
        eprintln!("SKIP ts_04_13: DATA_BROKER not reachable at {BROKER_ADDR}");
        return;
    }

    let vin = "REG-VIN";
    let nats = async_nats::connect(NATS_URL)
        .await
        .expect("Connect to NATS");

    // Subscribe BEFORE spawning so the subscription is in place before the
    // registration message is published (fire-and-forget per REQ-4.2).
    let mut sub = subscribe_for_registration(&nats, vin).await;

    let _guard = spawn_service(vin);

    let msg = recv_with_timeout(&mut sub, 5)
        .await
        .expect("ts_04_13: service should publish registration within 5 seconds");

    let payload =
        std::str::from_utf8(&msg.payload).expect("ts_04_13: registration payload must be valid UTF-8");
    let json: serde_json::Value =
        serde_json::from_str(payload).expect("ts_04_13: registration must be valid JSON");

    assert_eq!(
        json["vin"], vin,
        "ts_04_13: registration must contain correct vin"
    );
    assert_eq!(
        json["status"], "online",
        "ts_04_13: registration status must be 'online'"
    );
    assert!(
        json["timestamp"].is_number(),
        "ts_04_13: registration must contain a numeric timestamp"
    );
}

/// TS-04-14: Command rejected with invalid bearer token.
///
/// Validates: [04-REQ-5.E2]
///
/// Publishes a NATS command with the wrong bearer token and verifies that
/// `Vehicle.Command.Door.Lock` in DATA_BROKER is NOT updated with the rejected
/// command payload.
#[tokio::test]
#[ignore]
async fn ts_04_14_command_rejected_with_invalid_token() {
    if !nats_available().await {
        eprintln!("SKIP ts_04_14: NATS not reachable at {NATS_URL}");
        return;
    }
    if !broker_available().await {
        eprintln!("SKIP ts_04_14: DATA_BROKER not reachable at {BROKER_ADDR}");
        return;
    }

    let vin = "E2E-AUTH-VIN";
    let nats = async_nats::connect(NATS_URL)
        .await
        .expect("Connect to NATS");

    // Subscribe to registration before spawning.
    let mut reg_sub = subscribe_for_registration(&nats, vin).await;
    let _guard = spawn_service(vin);

    // Wait for service readiness.
    recv_with_timeout(&mut reg_sub, 10)
        .await
        .expect("ts_04_14: service should publish registration within 10 seconds");

    // Give the service a moment to complete DATA_BROKER subscriptions.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Write a sentinel value to the lock signal so we can detect any overwrite.
    let sentinel = r#"{"command_id":"sentinel-pre-auth","action":"lock","doors":[]}"#;
    write_actuator_string(SIGNAL_COMMAND_LOCK, sentinel).await;

    // Publish a command with the WRONG bearer token.
    let bad_payload = r#"{"command_id":"cmd-invalid-auth-14","action":"lock","doors":["driver"]}"#;
    let command_subject = format!("vehicles.{}.commands", vin);

    let mut headers = async_nats::HeaderMap::new();
    headers.insert("Authorization", "Bearer wrong-token");

    nats.publish_with_headers(
        command_subject,
        headers,
        bad_payload.as_bytes().to_vec().into(),
    )
    .await
    .expect("Publish rejected command to NATS");

    // Allow time for the service to process (or discard) the message.
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify DATA_BROKER was NOT updated with the bad command.
    let val = read_signal_string(SIGNAL_COMMAND_LOCK).await;
    match val {
        Some(v) => {
            assert!(
                !v.contains("cmd-invalid-auth-14"),
                "ts_04_14: DATA_BROKER must NOT be updated with the rejected command; got: {v}"
            );
        }
        None => {
            // Signal has no value → it was definitely not written. Pass.
        }
    }
}

// ── Smoke Tests ───────────────────────────────────────────────────────────────

/// TS-04-SMOKE-1: Service starts with valid configuration.
///
/// Validates: [04-REQ-2.1], [04-REQ-3.1]
///
/// Spawns the service with valid environment variables pointing to running NATS
/// and DATA_BROKER containers. Verifies that the process starts without error
/// (remains running for at least 2 seconds) and that INFO logs confirm
/// successful connections to both NATS and DATA_BROKER.
#[tokio::test]
#[ignore]
async fn ts_04_smoke_1_service_starts_with_valid_config() {
    if !nats_available().await {
        eprintln!("SKIP ts_04_smoke_1: NATS not reachable at {NATS_URL}");
        return;
    }
    if !broker_available().await {
        eprintln!("SKIP ts_04_smoke_1: DATA_BROKER not reachable at {BROKER_ADDR}");
        return;
    }

    let vin = "SMOKE-1-VIN";

    // Redirect both stdout and stderr to a temp file so we can inspect logs.
    let log_path = std::env::temp_dir().join("cgc-smoke-1.log");
    let log_file = std::fs::File::create(&log_path).expect("Create smoke-1 log file");
    let log_copy = log_file.try_clone().expect("Clone smoke-1 log file handle");

    let mut child = Command::new(env!("CARGO_BIN_EXE_cloud-gateway-client"))
        .env("VIN", vin)
        .env("NATS_URL", NATS_URL)
        .env("DATABROKER_ADDR", BROKER_ADDR)
        .env("BEARER_TOKEN", BEARER_TOKEN)
        .env("RUST_LOG", "info")
        .stdout(log_file)
        .stderr(log_copy)
        .spawn()
        .expect("ts_04_smoke_1: Failed to spawn binary");

    // Allow 4 seconds for the service to establish connections and log.
    tokio::time::sleep(Duration::from_secs(4)).await;

    // The process must still be running (no immediate error).
    match child.try_wait() {
        Ok(Some(status)) => {
            // Kill guard and panic with info.
            child.kill().ok();
            child.wait().ok();
            let log_content = std::fs::read_to_string(&log_path).unwrap_or_default();
            std::fs::remove_file(&log_path).ok();
            panic!(
                "ts_04_smoke_1: service exited unexpectedly with status: {status:?}\nLog:\n{log_content}"
            );
        }
        Ok(None) => {} // still running — expected
        Err(e) => {
            child.kill().ok();
            child.wait().ok();
            std::fs::remove_file(&log_path).ok();
            panic!("ts_04_smoke_1: error checking child status: {e}");
        }
    }

    // Terminate the service.
    child.kill().ok();
    child.wait().ok();

    // Inspect captured logs.
    let log_content = std::fs::read_to_string(&log_path).unwrap_or_default();
    std::fs::remove_file(&log_path).ok();

    assert!(
        log_content.contains("Connected to NATS"),
        "ts_04_smoke_1: logs must contain 'Connected to NATS'; got:\n{log_content}"
    );
    assert!(
        log_content.contains("Connected to DATA_BROKER"),
        "ts_04_smoke_1: logs must contain 'Connected to DATA_BROKER'; got:\n{log_content}"
    );
}

/// TS-04-SMOKE-2: Service exits with code 1 when VIN is not set.
///
/// Validates: [04-REQ-1.E1]
///
/// Runs the binary without VIN in the environment and verifies that the process
/// exits with a non-zero exit code and that the error output references "VIN".
///
/// This test does NOT require NATS or DATA_BROKER containers — the service
/// exits during configuration validation before any network connections.
#[test]
fn ts_04_smoke_2_service_exits_on_missing_vin() {
    let output = Command::new(env!("CARGO_BIN_EXE_cloud-gateway-client"))
        // Explicitly remove VIN from the environment so the service cannot find it.
        .env_remove("VIN")
        .env("NATS_URL", "nats://localhost:4222")
        .env("DATABROKER_ADDR", "http://localhost:55556")
        .env("BEARER_TOKEN", "demo-token")
        .output()
        .expect("ts_04_smoke_2: Failed to run cloud-gateway-client binary");

    assert!(
        !output.status.success(),
        "ts_04_smoke_2: service must exit with non-zero code when VIN is missing"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("VIN"),
        "ts_04_smoke_2: output must reference 'VIN'; got stderr='{stderr}' stdout='{stdout}'"
    );
}

/// TS-04-SMOKE-3: Service publishes registration message on startup.
///
/// Validates: [04-REQ-4.1]
///
/// Subscribes to `vehicles.SMOKE-3-VIN.status` before starting the service and
/// verifies that within 5 seconds a registration message is received with the
/// correct VIN and "status":"online".
#[tokio::test]
#[ignore]
async fn ts_04_smoke_3_service_publishes_registration_on_startup() {
    if !nats_available().await {
        eprintln!("SKIP ts_04_smoke_3: NATS not reachable at {NATS_URL}");
        return;
    }
    if !broker_available().await {
        eprintln!("SKIP ts_04_smoke_3: DATA_BROKER not reachable at {BROKER_ADDR}");
        return;
    }

    let vin = "SMOKE-3-VIN";
    let nats = async_nats::connect(NATS_URL)
        .await
        .expect("Connect to NATS");

    // Subscribe BEFORE spawning so the subscription is in place before the
    // registration message is published (fire-and-forget per REQ-4.2).
    let mut sub = subscribe_for_registration(&nats, vin).await;

    let _guard = spawn_service(vin);

    let msg = recv_with_timeout(&mut sub, 5)
        .await
        .expect("ts_04_smoke_3: service should publish registration within 5 seconds");

    let payload = std::str::from_utf8(&msg.payload)
        .expect("ts_04_smoke_3: registration payload must be valid UTF-8");
    let json: serde_json::Value =
        serde_json::from_str(payload).expect("ts_04_smoke_3: registration must be valid JSON");

    assert_eq!(json["vin"], vin, "ts_04_smoke_3: registration must contain the correct VIN");
    assert_eq!(
        json["status"], "online",
        "ts_04_smoke_3: registration status must be 'online'"
    );
    assert!(
        json["timestamp"].is_number(),
        "ts_04_smoke_3: registration must contain a numeric timestamp"
    );
}

/// TS-04-15: NATS reconnection with exponential backoff.
///
/// Validates: [04-REQ-2.2], [04-REQ-2.E1]
///
/// Starts the service pointing at a dead NATS port (no server listening) and
/// verifies that:
/// 1. The service exhausts all 5 connection attempts using exponential backoff
///    (wait intervals: 1s, 2s, 4s, 8s → total elapsed ≈ 15s).
/// 2. The service exits with a non-zero exit code after all retries fail.
///
/// **Note:** This test takes approximately 15 seconds to complete because the
/// exponential backoff schedule is real-time.
#[test]
#[ignore]
fn ts_04_15_nats_reconnection_exponential_backoff() {
    // Use a port very unlikely to have any server listening.
    let dead_nats_url = "nats://127.0.0.1:19222";

    let start = std::time::Instant::now();

    // Spawn the service; it will try to connect to the dead NATS URL.
    let mut child = Command::new(env!("CARGO_BIN_EXE_cloud-gateway-client"))
        .env("VIN", "BACKOFF-VIN")
        .env("NATS_URL", dead_nats_url)
        .env("DATABROKER_ADDR", BROKER_ADDR)
        .env("BEARER_TOKEN", BEARER_TOKEN)
        .env("RUST_LOG", "warn")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("ts_04_15: Failed to spawn cloud-gateway-client binary");

    // The service should exit after ≈15s of backoff (1+2+4+8 seconds of waits
    // between 5 failed connection attempts). Give it up to 35 seconds.
    let deadline = std::time::Duration::from_secs(35);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if start.elapsed() > deadline {
                    child.kill().ok();
                    child.wait().ok();
                    panic!(
                        "ts_04_15: service did not exit within {deadline:?} — backoff may not be terminating"
                    );
                }
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            Err(e) => {
                child.kill().ok();
                child.wait().ok();
                panic!("ts_04_15: error waiting for child process: {e}");
            }
        }
    };

    let elapsed = start.elapsed();

    // Service must exit with a non-zero code after exhausting NATS retries.
    assert!(
        !status.success(),
        "ts_04_15: service must exit with non-zero code after NATS retries exhausted"
    );

    // The full backoff schedule totals 1+2+4+8 = 15 seconds.
    // Allow 8–30 seconds as a generous tolerance window.
    assert!(
        elapsed >= std::time::Duration::from_secs(8),
        "ts_04_15: service exited too quickly ({elapsed:?}); expected ≈15s for full backoff"
    );
    assert!(
        elapsed <= std::time::Duration::from_secs(30),
        "ts_04_15: service took too long ({elapsed:?}); expected ≈15s for full backoff"
    );
}
