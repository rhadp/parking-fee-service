//! Integration tests for CLOUD_GATEWAY_CLIENT
//!
//! These tests verify end-to-end data flows through NATS and DATA_BROKER:
//! - TS-04-10: Command forwarded from NATS to DATA_BROKER
//! - TS-04-11: Response relayed from DATA_BROKER to NATS
//! - TS-04-12: Telemetry published on signal change in DATA_BROKER
//! - TS-04-13: Self-registration published on startup
//! - TS-04-14: Command with wrong bearer token is rejected
//!
//! **Prerequisites:** NATS on localhost:4222 and DATA_BROKER on localhost:55556.
//! Tests skip gracefully when infrastructure is not available.
//!
//! Run with:
//!   cargo test -p cloud-gateway-client -- --ignored
//!
//! Start infrastructure with:
//!   cd deployments && podman compose up -d

use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

use futures::StreamExt as _;

// Include the generated proto bindings so we can talk directly to DATA_BROKER.
#[allow(clippy::enum_variant_names)]
mod kuksa_val_v1 {
    tonic::include_proto!("kuksa.val.v1");
}

use kuksa_val_v1::datapoint::Value as DatapointValue;
use kuksa_val_v1::val_client::ValClient;
use kuksa_val_v1::{
    DataEntry, Datapoint, EntryRequest, EntryUpdate, Field, GetRequest, SetRequest, View,
};

// ── Constants ─────────────────────────────────────────────────────────────────

const NATS_URL: &str = "nats://localhost:4222";
const BROKER_ADDR: &str = "http://localhost:55556";
const BEARER_TOKEN: &str = "demo-token";

// ── Infrastructure helpers ─────────────────────────────────────────────────────

/// Try to connect to the NATS server within 2 seconds.
/// Returns `None` if NATS is not reachable.
async fn try_nats() -> Option<async_nats::Client> {
    timeout(Duration::from_secs(2), async_nats::connect(NATS_URL))
        .await
        .ok()
        .and_then(|r| r.ok())
}

/// Try to connect to the DATA_BROKER within 2 seconds.
/// Returns `None` if DATA_BROKER is not reachable.
async fn try_broker() -> Option<ValClient<tonic::transport::Channel>> {
    timeout(
        Duration::from_secs(2),
        ValClient::connect(BROKER_ADDR.to_string()),
    )
    .await
    .ok()
    .and_then(|r| r.ok())
}

/// Spawn the cloud-gateway-client binary configured with the given VIN.
/// All other settings use the test-local infrastructure addresses.
/// The caller is responsible for killing the child process when done.
fn start_service(vin: &str) -> Child {
    Command::new(env!("CARGO_BIN_EXE_cloud-gateway-client"))
        .env("VIN", vin)
        .env("NATS_URL", NATS_URL)
        .env("DATABROKER_ADDR", BROKER_ADDR)
        .env("BEARER_TOKEN", BEARER_TOKEN)
        .env("RUST_LOG", "warn")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn cloud-gateway-client binary")
}

/// Wait up to `secs` seconds for the next message on the subscriber.
async fn wait_for_message(
    sub: &mut async_nats::Subscriber,
    secs: u64,
) -> Option<async_nats::Message> {
    timeout(Duration::from_secs(secs), sub.next())
        .await
        .ok()
        .flatten()
}

/// Write a string value to a DATA_BROKER VSS signal path via gRPC Set.
async fn broker_set_string(
    client: &mut ValClient<tonic::transport::Channel>,
    path: &str,
    value: &str,
) {
    let request = SetRequest {
        updates: vec![EntryUpdate {
            entry: Some(DataEntry {
                path: path.to_string(),
                value: Some(Datapoint {
                    timestamp: 0,
                    value: Some(DatapointValue::StringValue(value.to_string())),
                }),
                actuator_target: None,
                metadata: None,
            }),
            fields: vec![Field::Value as i32],
        }],
    };
    // Ignore write errors — if DATA_BROKER doesn't know the path yet (e.g.,
    // custom VSS overlay not applied), the test will timeout rather than panic.
    client.clone().set(request).await.ok();
}

/// Write a boolean value to a DATA_BROKER VSS signal path via gRPC Set.
async fn broker_set_bool(
    client: &mut ValClient<tonic::transport::Channel>,
    path: &str,
    value: bool,
) {
    let request = SetRequest {
        updates: vec![EntryUpdate {
            entry: Some(DataEntry {
                path: path.to_string(),
                value: Some(Datapoint {
                    timestamp: 0,
                    value: Some(DatapointValue::BoolValue(value)),
                }),
                actuator_target: None,
                metadata: None,
            }),
            fields: vec![Field::Value as i32],
        }],
    };
    client.clone().set(request).await.ok();
}

/// Read the current string value of a DATA_BROKER VSS signal path via gRPC Get.
/// Returns `None` if the signal is not set or on any error.
async fn broker_get_string(
    client: &mut ValClient<tonic::transport::Channel>,
    path: &str,
) -> Option<String> {
    let request = GetRequest {
        entries: vec![EntryRequest {
            path: path.to_string(),
            view: View::CurrentValue as i32,
            fields: vec![Field::Value as i32],
        }],
    };
    let response = client.clone().get(request).await.ok()?;
    let entry = response.into_inner().entries.into_iter().next()?;
    match entry.value?.value? {
        DatapointValue::StringValue(s) => Some(s),
        _ => None,
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

/// TS-04-10: End-to-end command flow.
///
/// A valid lock command published on NATS with the correct bearer token is
/// forwarded verbatim to `Vehicle.Command.Door.Lock` in DATA_BROKER within 2 s.
///
/// Validates [04-REQ-2.3], [04-REQ-5.2], [04-REQ-6.3]
#[tokio::test]
#[ignore = "integration test — requires running NATS and DATA_BROKER containers"]
async fn ts_04_10_end_to_end_command_flow() {
    let nats = match try_nats().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP: NATS not available at {NATS_URL}");
            return;
        }
    };
    let mut broker = match try_broker().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP: DATA_BROKER not available at {BROKER_ADDR}");
            return;
        }
    };

    let vin = "E2E-10";

    // Subscribe to the registration subject BEFORE starting the service so we
    // don't miss the startup announcement.
    let mut reg_sub = nats
        .subscribe(format!("vehicles.{vin}.status"))
        .await
        .expect("subscribe to status failed");

    let mut child = start_service(vin);

    // Wait for the service to announce itself (proves connections are up).
    if wait_for_message(&mut reg_sub, 5).await.is_none() {
        child.kill().ok();
        panic!("Service did not publish registration within 5 s");
    }

    // Publish a lock command with the correct bearer token.
    // Use a unique command_id so the assertion can't be fooled by stale state.
    let command_payload =
        r#"{"command_id":"cmd-e2e-10","action":"lock","doors":["driver"],"source":"companion_app","vin":"E2E-10","timestamp":1700000000}"#;

    let mut headers = async_nats::HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {BEARER_TOKEN}").as_str(),
    );
    nats.publish_with_headers(
        format!("vehicles.{vin}.commands"),
        headers,
        command_payload.to_owned().into(),
    )
    .await
    .expect("Failed to publish NATS command");

    // Poll DATA_BROKER for up to 2 s to verify the payload was written verbatim.
    let verified = timeout(Duration::from_secs(2), async {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if let Some(val) =
                broker_get_string(&mut broker, "Vehicle.Command.Door.Lock").await
            {
                if val == command_payload {
                    return true;
                }
            }
        }
    })
    .await
    .unwrap_or(false);

    child.kill().ok();
    child.wait().ok();

    assert!(
        verified,
        "Vehicle.Command.Door.Lock was not updated with the expected payload within 2 s"
    );
}

/// TS-04-11: End-to-end response relay.
///
/// A response written to `Vehicle.Command.Door.Response` in DATA_BROKER is
/// published verbatim to `vehicles.{VIN}.command_responses` on NATS within 2 s.
///
/// Validates [04-REQ-7.1], [04-REQ-7.2]
#[tokio::test]
#[ignore = "integration test — requires running NATS and DATA_BROKER containers"]
async fn ts_04_11_end_to_end_response_relay() {
    let nats = match try_nats().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP: NATS not available at {NATS_URL}");
            return;
        }
    };
    let mut broker = match try_broker().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP: DATA_BROKER not available at {BROKER_ADDR}");
            return;
        }
    };

    let vin = "E2E-11";

    // Subscribe to the registration subject BEFORE starting the service.
    let mut reg_sub = nats
        .subscribe(format!("vehicles.{vin}.status"))
        .await
        .expect("subscribe to status failed");

    let mut child = start_service(vin);

    if wait_for_message(&mut reg_sub, 5).await.is_none() {
        child.kill().ok();
        panic!("Service did not publish registration within 5 s");
    }

    // Subscribe to NATS command_responses BEFORE writing to DATA_BROKER so we
    // don't miss the relay triggered by the write.
    let mut rsp_sub = nats
        .subscribe(format!("vehicles.{vin}.command_responses"))
        .await
        .expect("subscribe to command_responses failed");

    // Write a response value to DATA_BROKER.
    let response_json =
        r#"{"command_id":"cmd-e2e-11","status":"success","timestamp":1700000001}"#;
    broker_set_string(
        &mut broker,
        "Vehicle.Command.Door.Response",
        response_json,
    )
    .await;

    // Wait up to 2 s for the relay to appear on NATS.
    // If the service received the initial value on subscription it may send that
    // first; we drain until we see our unique command_id.
    let verified = timeout(Duration::from_secs(2), async {
        while let Some(msg) = rsp_sub.next().await {
            if let Ok(s) = std::str::from_utf8(&msg.payload) {
                if s == response_json {
                    return true;
                }
            }
        }
        false
    })
    .await
    .unwrap_or(false);

    child.kill().ok();
    child.wait().ok();

    assert!(
        verified,
        "Expected response JSON was not relayed to NATS command_responses within 2 s"
    );
}

/// TS-04-12: End-to-end telemetry on signal change.
///
/// When `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` is set in DATA_BROKER,
/// the service publishes a telemetry JSON to `vehicles.{VIN}.telemetry` on NATS
/// containing `vin` and `is_locked: true`.
///
/// Validates [04-REQ-8.1], [04-REQ-8.2]
#[tokio::test]
#[ignore = "integration test — requires running NATS and DATA_BROKER containers"]
async fn ts_04_12_end_to_end_telemetry_on_signal_change() {
    let nats = match try_nats().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP: NATS not available at {NATS_URL}");
            return;
        }
    };
    let mut broker = match try_broker().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP: DATA_BROKER not available at {BROKER_ADDR}");
            return;
        }
    };

    let vin = "E2E-12";

    // Subscribe to the registration subject BEFORE starting the service.
    let mut reg_sub = nats
        .subscribe(format!("vehicles.{vin}.status"))
        .await
        .expect("subscribe to status failed");

    let mut child = start_service(vin);

    if wait_for_message(&mut reg_sub, 5).await.is_none() {
        child.kill().ok();
        panic!("Service did not publish registration within 5 s");
    }

    // Subscribe to NATS telemetry BEFORE writing to DATA_BROKER.
    let mut tel_sub = nats
        .subscribe(format!("vehicles.{vin}.telemetry"))
        .await
        .expect("subscribe to telemetry failed");

    // Set IsLocked = true in DATA_BROKER to trigger a telemetry publish.
    broker_set_bool(
        &mut broker,
        "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
        true,
    )
    .await;

    // Drain messages until we see one from our VIN with is_locked: true,
    // or until the 2 s timeout expires.
    let verified = timeout(Duration::from_secs(2), async {
        while let Some(msg) = tel_sub.next().await {
            if let Ok(s) = std::str::from_utf8(&msg.payload) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(s) {
                    if v["vin"] == vin && v["is_locked"] == true {
                        return true;
                    }
                }
            }
        }
        false
    })
    .await
    .unwrap_or(false);

    child.kill().ok();
    child.wait().ok();

    assert!(
        verified,
        "Expected telemetry JSON with is_locked:true was not received on NATS within 2 s"
    );
}

/// TS-04-13: Self-registration on startup.
///
/// When CLOUD_GATEWAY_CLIENT starts it publishes a registration message to
/// `vehicles.{VIN}.status` within 5 seconds containing `vin`, `status:"online"`,
/// and a numeric `timestamp`.
///
/// Validates [04-REQ-4.1], [04-REQ-4.2]
#[tokio::test]
#[ignore = "integration test — requires running NATS and DATA_BROKER containers"]
async fn ts_04_13_self_registration_on_startup() {
    let nats = match try_nats().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP: NATS not available at {NATS_URL}");
            return;
        }
    };
    // DATA_BROKER must be reachable for the service to complete its startup sequence.
    if try_broker().await.is_none() {
        eprintln!("SKIP: DATA_BROKER not available at {BROKER_ADDR}");
        return;
    }

    let vin = "REG-13";

    // Subscribe BEFORE starting the service so we cannot miss the message.
    let mut sub = nats
        .subscribe(format!("vehicles.{vin}.status"))
        .await
        .expect("subscribe to status failed");

    let mut child = start_service(vin);

    // Wait up to 5 s for the registration message.
    let msg = wait_for_message(&mut sub, 5).await;

    child.kill().ok();
    child.wait().ok();

    let msg = msg.expect("Registration message not received within 5 s");
    let payload = std::str::from_utf8(&msg.payload).expect("payload is not UTF-8");
    let parsed: serde_json::Value =
        serde_json::from_str(payload).expect("registration payload is not valid JSON");

    assert_eq!(parsed["vin"], vin, "vin field mismatch in registration message");
    assert_eq!(
        parsed["status"], "online",
        "status field must be 'online'"
    );
    assert!(
        parsed["timestamp"].is_number(),
        "timestamp must be a number"
    );
}

/// TS-04-14: Command rejected with invalid bearer token.
///
/// A NATS command with a wrong bearer token is discarded by the service; the
/// unique payload is NOT written to `Vehicle.Command.Door.Lock` in DATA_BROKER.
///
/// Validates [04-REQ-5.E2]
#[tokio::test]
#[ignore = "integration test — requires running NATS and DATA_BROKER containers"]
async fn ts_04_14_command_rejected_with_invalid_token() {
    let nats = match try_nats().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP: NATS not available at {NATS_URL}");
            return;
        }
    };
    let mut broker = match try_broker().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP: DATA_BROKER not available at {BROKER_ADDR}");
            return;
        }
    };

    let vin = "E2E-14";

    // Subscribe to the registration subject BEFORE starting the service.
    let mut reg_sub = nats
        .subscribe(format!("vehicles.{vin}.status"))
        .await
        .expect("subscribe to status failed");

    let mut child = start_service(vin);

    if wait_for_message(&mut reg_sub, 5).await.is_none() {
        child.kill().ok();
        panic!("Service did not publish registration within 5 s");
    }

    // Publish a command with a WRONG bearer token.
    // Use a unique command_id (cmd-e2e-14) so we can detect if it appears.
    let command_payload =
        r#"{"command_id":"cmd-e2e-14","action":"lock","doors":["driver"]}"#;
    let mut headers = async_nats::HeaderMap::new();
    headers.insert("Authorization", "Bearer wrong-token");
    nats.publish_with_headers(
        format!("vehicles.{vin}.commands"),
        headers,
        command_payload.to_owned().into(),
    )
    .await
    .expect("Failed to publish NATS command");

    // Give the service 2 s to process the command (it should discard it).
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Verify that the command payload did NOT reach DATA_BROKER.
    let current = broker_get_string(&mut broker, "Vehicle.Command.Door.Lock").await;

    child.kill().ok();
    child.wait().ok();

    assert_ne!(
        current,
        Some(command_payload.to_string()),
        "Vehicle.Command.Door.Lock was incorrectly updated despite wrong bearer token"
    );
}
