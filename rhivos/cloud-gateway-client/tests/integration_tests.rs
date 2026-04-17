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
#[allow(clippy::enum_variant_names)]
mod kuksa {
    tonic::include_proto!("kuksa");
}

use kuksa::val_service_client::ValServiceClient;
use kuksa::{DataEntry, Datapoint, Field, GetRequest, SetRequest};
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
async fn raw_broker() -> ValServiceClient<Channel> {
    ValServiceClient::connect(BROKER_ADDR)
        .await
        .expect("Test helper: failed to connect to DATA_BROKER")
}

/// Read the current string value of a VSS signal from DATA_BROKER.
///
/// Returns `None` if the signal was never set or carries a non-string value.
async fn read_signal_string(path: &str) -> Option<String> {
    let mut client = raw_broker().await;
    let resp = client
        .get(GetRequest {
            entries: vec![Field {
                path: path.to_owned(),
            }],
        })
        .await
        .ok()?
        .into_inner();

    for entry in resp.entries {
        if entry.path == path {
            if let Some(dp) = entry.value {
                if let Some(kuksa::datapoint::Value::StringValue(s)) = dp.value {
                    return Some(s);
                }
            }
        }
    }
    None
}

/// Write a string value to a VSS signal in DATA_BROKER.
async fn write_signal_string(path: &str, value: &str) {
    let mut client = raw_broker().await;
    client
        .set(SetRequest {
            updates: vec![DataEntry {
                path: path.to_owned(),
                value: Some(Datapoint {
                    timestamp: 0,
                    value: Some(kuksa::datapoint::Value::StringValue(value.to_owned())),
                }),
            }],
        })
        .await
        .expect("Test helper: failed to write string signal to DATA_BROKER");
}

/// Write a boolean value to a VSS signal in DATA_BROKER.
async fn write_signal_bool(path: &str, value: bool) {
    let mut client = raw_broker().await;
    client
        .set(SetRequest {
            updates: vec![DataEntry {
                path: path.to_owned(),
                value: Some(Datapoint {
                    timestamp: 0,
                    value: Some(kuksa::datapoint::Value::BoolValue(value)),
                }),
            }],
        })
        .await
        .expect("Test helper: failed to write bool signal to DATA_BROKER");
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
    tokio::time::sleep(Duration::from_millis(200)).await;

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

    // Write a command response to DATA_BROKER.
    let response_json =
        r#"{"command_id":"cmd-relay-11","status":"success","timestamp":1700000001}"#;
    write_signal_string(SIGNAL_COMMAND_RESPONSE, response_json).await;

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
    write_signal_bool(SIGNAL_IS_LOCKED, true).await;

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
    write_signal_string(SIGNAL_COMMAND_LOCK, sentinel).await;

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
