//! Integration tests for CLOUD_GATEWAY_CLIENT.
//!
//! These tests exercise end-to-end data flows through real NATS and
//! DATA_BROKER infrastructure.
//!
//! **Prerequisites:** Start containers before running:
//! ```sh
//! podman-compose -f deployments/compose.yml up -d
//! ```
//!
//! **Run:** `cargo test -p cloud-gateway-client -- --ignored`
//!
//! Test Specifications:
//! - TS-04-10: End-to-end command flow
//! - TS-04-11: End-to-end response relay
//! - TS-04-12: End-to-end telemetry on signal change
//! - TS-04-13: Self-registration on startup
//! - TS-04-14: Command rejected with invalid token

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use tokio::time::timeout;

use cloud_gateway_client::broker_client::kuksa::val::v2::{
    signal_id::Signal, val_client::ValClient, value::TypedValue, Datapoint, GetValueRequest,
    PublishValueRequest, SignalId, Value,
};
use cloud_gateway_client::broker_client::BrokerClient;
use cloud_gateway_client::command_validator;
use cloud_gateway_client::config::Config;
use cloud_gateway_client::nats_client::NatsClient;
use cloud_gateway_client::telemetry::TelemetryState;

const NATS_URL: &str = "nats://localhost:4222";
const DATABROKER_ADDR: &str = "http://localhost:55556";
const BEARER_TOKEN: &str = "demo-token";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Creates a [`Config`] for integration testing with the given VIN.
fn test_config(vin: &str) -> Config {
    Config {
        vin: vin.to_string(),
        nats_url: NATS_URL.to_string(),
        databroker_addr: DATABROKER_ADDR.to_string(),
        bearer_token: BEARER_TOKEN.to_string(),
    }
}

/// Connects a raw `ValClient` to DATA_BROKER for direct gRPC verification.
async fn raw_val_client() -> ValClient<tonic::transport::Channel> {
    ValClient::connect(DATABROKER_ADDR)
        .await
        .expect("failed to connect raw ValClient to DATA_BROKER")
}

/// Reads a string-typed signal from DATA_BROKER via `GetValue` RPC.
///
/// Returns `None` if the signal has no value or is not string-typed.
async fn get_string_signal(
    client: &mut ValClient<tonic::transport::Channel>,
    path: &str,
) -> Option<String> {
    let req = GetValueRequest {
        signal_id: Some(SignalId {
            signal: Some(Signal::Path(path.to_string())),
        }),
    };
    let resp = client.get_value(req).await.ok()?;
    let dp = resp.into_inner().data_point?;
    let val = dp.value?;
    match val.typed_value? {
        TypedValue::String(s) => Some(s),
        _ => None,
    }
}

/// Writes a string-typed signal to DATA_BROKER via `PublishValue` RPC.
async fn set_string_signal(
    client: &mut ValClient<tonic::transport::Channel>,
    path: &str,
    value: &str,
) {
    let req = PublishValueRequest {
        signal_id: Some(SignalId {
            signal: Some(Signal::Path(path.to_string())),
        }),
        data_point: Some(Datapoint {
            timestamp: None,
            value: Some(Value {
                typed_value: Some(TypedValue::String(value.to_string())),
            }),
        }),
    };
    client
        .publish_value(req)
        .await
        .expect("failed to write string signal to DATA_BROKER");
}

/// Writes a bool-typed signal to DATA_BROKER via `PublishValue` RPC.
async fn set_bool_signal(
    client: &mut ValClient<tonic::transport::Channel>,
    path: &str,
    value: bool,
) {
    let req = PublishValueRequest {
        signal_id: Some(SignalId {
            signal: Some(Signal::Path(path.to_string())),
        }),
        data_point: Some(Datapoint {
            timestamp: None,
            value: Some(Value {
                typed_value: Some(TypedValue::Bool(value)),
            }),
        }),
    };
    client
        .publish_value(req)
        .await
        .expect("failed to write bool signal to DATA_BROKER");
}

/// Waits for a NATS message whose payload satisfies `predicate`.
///
/// Skips non-matching messages (e.g., stale values from prior runs or
/// initial subscription deliveries). Returns `None` if the timeout expires
/// before a matching message arrives.
async fn wait_for_matching_message(
    subscriber: &mut async_nats::Subscriber,
    timeout_duration: Duration,
    predicate: impl Fn(&[u8]) -> bool,
) -> Option<async_nats::Message> {
    let start = tokio::time::Instant::now();
    loop {
        let elapsed = start.elapsed();
        if elapsed >= timeout_duration {
            return None;
        }
        let remaining = timeout_duration - elapsed;
        match timeout(remaining, subscriber.next()).await {
            Ok(Some(msg)) => {
                if predicate(&msg.payload) {
                    return Some(msg);
                }
                // Skip non-matching messages.
            }
            Ok(None) | Err(_) => return None,
        }
    }
}

// ---------------------------------------------------------------------------
// Service loop replicas
//
// The async processing loops live in `main.rs` (binary crate) and are not
// part of the library's public API. Integration tests replicate the logic
// here to exercise the library components end-to-end.
// ---------------------------------------------------------------------------

/// Runs the command processing loop: NATS commands -> validate -> DATA_BROKER.
///
/// Replicates the `command_loop` from `main.rs`.
async fn command_processing_loop(
    nats: Arc<NatsClient>,
    broker: Arc<BrokerClient>,
    bearer_token: String,
) {
    let mut subscriber = nats
        .subscribe_commands()
        .await
        .expect("subscribe_commands failed in test loop");

    while let Some(message) = subscriber.next().await {
        let auth_header: Option<&str> = message
            .headers
            .as_ref()
            .and_then(|h| h.get("Authorization"))
            .map(|v| v.as_str());

        if command_validator::validate_bearer_token(auth_header, &bearer_token).is_err() {
            continue;
        }

        if command_validator::validate_command_payload(&message.payload).is_err() {
            continue;
        }

        if let Ok(payload_str) = std::str::from_utf8(&message.payload) {
            let _ = broker.write_command(payload_str).await;
        }
    }
}

/// Runs the response relay loop: DATA_BROKER responses -> NATS.
///
/// Replicates the `response_relay_loop` from `main.rs`.
async fn response_relay_loop(nats: Arc<NatsClient>, broker: Arc<BrokerClient>) {
    let mut rx = broker
        .subscribe_responses()
        .await
        .expect("subscribe_responses failed in test loop");

    while let Some(response_json) = rx.recv().await {
        let _ = nats.publish_response(&response_json).await;
    }
}

/// Runs the telemetry publishing loop: DATA_BROKER signals -> aggregate -> NATS.
///
/// Replicates the `telemetry_loop` from `main.rs`.
async fn telemetry_publishing_loop(
    nats: Arc<NatsClient>,
    broker: Arc<BrokerClient>,
    vin: String,
) {
    let mut rx = broker
        .subscribe_telemetry()
        .await
        .expect("subscribe_telemetry failed in test loop");

    let mut state = TelemetryState::new(vin);

    while let Some(signal_update) = rx.recv().await {
        if let Some(telemetry_json) = state.update(signal_update) {
            let _ = nats.publish_telemetry(&telemetry_json).await;
        }
    }
}

// ===========================================================================
// TS-04-10: End-to-end command flow
// Validates: [04-REQ-5.2], [04-REQ-6.3], [04-REQ-2.3]
// ===========================================================================

/// GIVEN CLOUD_GATEWAY_CLIENT is running with VIN="E2E-VIN"
/// GIVEN NATS subscriber is listening on DATA_BROKER for Vehicle.Command.Door.Lock
/// WHEN a NATS message is published to "vehicles.E2E-VIN.commands"
///   WITH header "Authorization" = "Bearer demo-token"
///   WITH payload containing command_id, action, doors
/// THEN within 2 seconds, Vehicle.Command.Door.Lock in DATA_BROKER
///   contains the command payload.
#[tokio::test]
#[ignore]
#[serial_test::serial]
async fn ts_04_10_end_to_end_command_flow() {
    let vin = "E2E-CMD-VIN";
    let config = test_config(vin);

    // Connect service components.
    let nats = Arc::new(NatsClient::connect(&config).await.unwrap());
    let broker = Arc::new(BrokerClient::connect(&config).await.unwrap());

    // Spawn command processing loop.
    let cmd_nats = Arc::clone(&nats);
    let cmd_broker = Arc::clone(&broker);
    let token = config.bearer_token.clone();
    tokio::spawn(command_processing_loop(cmd_nats, cmd_broker, token));

    // Allow the command loop to establish its NATS subscription.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Publish a valid command via a raw NATS connection.
    let raw_nats = async_nats::connect(NATS_URL).await.unwrap();

    let payload = r#"{"command_id":"cmd-1","action":"lock","doors":["driver"],"source":"companion_app","vin":"E2E-CMD-VIN","timestamp":1700000000}"#;

    let mut headers = async_nats::HeaderMap::new();
    headers.insert("Authorization", "Bearer demo-token");

    raw_nats
        .publish_with_headers(
            format!("vehicles.{}.commands", vin),
            headers,
            payload.into(),
        )
        .await
        .unwrap();
    raw_nats.flush().await.unwrap();

    // Wait for the command to be processed and written to DATA_BROKER.
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify the command was written to DATA_BROKER verbatim.
    let mut val_client = raw_val_client().await;
    let stored = get_string_signal(&mut val_client, "Vehicle.Command.Door.Lock").await;

    assert_eq!(
        stored.as_deref(),
        Some(payload),
        "Vehicle.Command.Door.Lock in DATA_BROKER should contain the command payload verbatim"
    );
}

// ===========================================================================
// TS-04-11: End-to-end response relay
// Validates: [04-REQ-7.1], [04-REQ-7.2]
// ===========================================================================

/// GIVEN CLOUD_GATEWAY_CLIENT is running with VIN="E2E-VIN"
/// GIVEN NATS subscriber is listening on "vehicles.E2E-VIN.command_responses"
/// WHEN Vehicle.Command.Door.Response is set to a response JSON in DATA_BROKER
/// THEN within 2 seconds, the NATS subscriber receives the response JSON verbatim.
#[tokio::test]
#[ignore]
#[serial_test::serial]
async fn ts_04_11_end_to_end_response_relay() {
    let vin = "E2E-RSP-VIN";
    let config = test_config(vin);

    // Connect service components.
    let nats = Arc::new(NatsClient::connect(&config).await.unwrap());
    let broker = Arc::new(BrokerClient::connect(&config).await.unwrap());

    // Subscribe to command_responses on NATS via raw client.
    let raw_nats = async_nats::connect(NATS_URL).await.unwrap();
    let mut response_sub = raw_nats
        .subscribe(format!("vehicles.{}.command_responses", vin))
        .await
        .unwrap();

    // Spawn response relay loop.
    let relay_nats = Arc::clone(&nats);
    let relay_broker = Arc::clone(&broker);
    tokio::spawn(response_relay_loop(relay_nats, relay_broker));

    // Allow subscriptions to settle (NATS + DATA_BROKER gRPC stream).
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Write a response JSON to Vehicle.Command.Door.Response in DATA_BROKER.
    let response_json =
        r#"{"command_id":"cmd-1","status":"success","timestamp":1700000001}"#;
    let mut val_client = raw_val_client().await;
    set_string_signal(
        &mut val_client,
        "Vehicle.Command.Door.Response",
        response_json,
    )
    .await;

    // Wait for the response to arrive on NATS (filter by exact payload to
    // skip stale values from prior runs or initial subscription deliveries).
    let msg = wait_for_matching_message(
        &mut response_sub,
        Duration::from_secs(2),
        |payload| payload == response_json.as_bytes(),
    )
    .await;

    let msg = msg.expect("should receive command response on NATS within 2s");
    let received = std::str::from_utf8(&msg.payload).expect("payload should be UTF-8");
    assert_eq!(
        received, response_json,
        "response should be relayed verbatim from DATA_BROKER to NATS"
    );
}

// ===========================================================================
// TS-04-12: End-to-end telemetry on signal change
// Validates: [04-REQ-8.1], [04-REQ-8.2]
// ===========================================================================

/// GIVEN CLOUD_GATEWAY_CLIENT is running with VIN="E2E-VIN"
/// GIVEN NATS subscriber is listening on "vehicles.E2E-VIN.telemetry"
/// WHEN Vehicle.Cabin.Door.Row1.DriverSide.IsLocked is set to true in DATA_BROKER
/// THEN within 2 seconds, the NATS subscriber receives a telemetry JSON
///   containing "vin":"E2E-VIN" and "is_locked":true.
#[tokio::test]
#[ignore]
#[serial_test::serial]
async fn ts_04_12_end_to_end_telemetry() {
    let vin = "E2E-TEL-VIN";
    let config = test_config(vin);

    // Connect service components.
    let nats = Arc::new(NatsClient::connect(&config).await.unwrap());
    let broker = Arc::new(BrokerClient::connect(&config).await.unwrap());

    // Subscribe to telemetry on NATS via raw client.
    let raw_nats = async_nats::connect(NATS_URL).await.unwrap();
    let mut telemetry_sub = raw_nats
        .subscribe(format!("vehicles.{}.telemetry", vin))
        .await
        .unwrap();

    // Spawn telemetry loop.
    let tel_nats = Arc::clone(&nats);
    let tel_broker = Arc::clone(&broker);
    let tel_vin = vin.to_string();
    tokio::spawn(telemetry_publishing_loop(tel_nats, tel_broker, tel_vin));

    // Allow subscriptions to settle.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Set IsLocked = true in DATA_BROKER.
    let mut val_client = raw_val_client().await;
    set_bool_signal(
        &mut val_client,
        "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
        true,
    )
    .await;

    // Wait for a telemetry message that contains our VIN and is_locked:true.
    let msg = wait_for_matching_message(
        &mut telemetry_sub,
        Duration::from_secs(2),
        |payload| {
            std::str::from_utf8(payload)
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .map(|v| {
                    v.get("vin").and_then(|f| f.as_str()) == Some(vin)
                        && v.get("is_locked").and_then(|f| f.as_bool()) == Some(true)
                })
                .unwrap_or(false)
        },
    )
    .await;

    let msg = msg.expect("should receive telemetry on NATS within 2s");
    let payload: serde_json::Value =
        serde_json::from_slice(&msg.payload).expect("telemetry should be valid JSON");

    assert_eq!(
        payload["vin"].as_str(),
        Some(vin),
        "telemetry should contain the correct VIN"
    );
    assert_eq!(
        payload["is_locked"].as_bool(),
        Some(true),
        "telemetry should contain is_locked:true"
    );
    assert!(
        payload.get("timestamp").is_some(),
        "telemetry should contain a timestamp"
    );
}

// ===========================================================================
// TS-04-13: Self-registration on startup
// Validates: [04-REQ-4.1], [04-REQ-4.2]
// ===========================================================================

/// GIVEN NATS subscriber is listening on "vehicles.REG-VIN.status"
/// WHEN CLOUD_GATEWAY_CLIENT is started with VIN="REG-VIN"
/// THEN within 5 seconds, the NATS subscriber receives a registration message
///   with "vin":"REG-VIN" and "status":"online".
#[tokio::test]
#[ignore]
#[serial_test::serial]
async fn ts_04_13_self_registration_on_startup() {
    let vin = "REG-VIN";

    // Subscribe to the status subject BEFORE starting the service so we
    // do not miss the fire-and-forget registration message.
    let raw_nats = async_nats::connect(NATS_URL).await.unwrap();
    let mut status_sub = raw_nats
        .subscribe(format!("vehicles.{}.status", vin))
        .await
        .unwrap();

    // Allow subscription to propagate to the NATS server.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Start the service's NatsClient and publish registration.
    let config = test_config(vin);
    let nats_client = NatsClient::connect(&config).await.unwrap();
    nats_client.publish_registration().await.unwrap();

    // Wait for the registration message.
    let msg = wait_for_matching_message(
        &mut status_sub,
        Duration::from_secs(5),
        |payload| {
            std::str::from_utf8(payload)
                .ok()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .map(|v| {
                    v.get("vin").and_then(|f| f.as_str()) == Some(vin)
                        && v.get("status").and_then(|f| f.as_str()) == Some("online")
                })
                .unwrap_or(false)
        },
    )
    .await;

    let msg = msg.expect("should receive registration message on NATS within 5s");
    let payload: serde_json::Value =
        serde_json::from_slice(&msg.payload).expect("registration should be valid JSON");

    assert_eq!(payload["vin"].as_str(), Some(vin));
    assert_eq!(payload["status"].as_str(), Some("online"));
    assert!(
        payload.get("timestamp").is_some(),
        "registration should contain a timestamp"
    );
}

// ===========================================================================
// TS-04-14: Command rejected with invalid token
// Validates: [04-REQ-5.E2]
// ===========================================================================

/// GIVEN CLOUD_GATEWAY_CLIENT is running with VIN="E2E-VIN"
/// WHEN a NATS message is published to "vehicles.E2E-VIN.commands"
///   WITH header "Authorization" = "Bearer wrong-token"
///   WITH payload containing a valid command
/// THEN Vehicle.Command.Door.Lock in DATA_BROKER is NOT updated
///   AND no message is published to "vehicles.E2E-VIN.command_responses".
#[tokio::test]
#[ignore]
#[serial_test::serial]
async fn ts_04_14_command_rejected_with_invalid_token() {
    let vin = "E2E-REJ-VIN";
    let config = test_config(vin);

    // Connect service components.
    let nats = Arc::new(NatsClient::connect(&config).await.unwrap());
    let broker = Arc::new(BrokerClient::connect(&config).await.unwrap());

    // Set a marker value in Vehicle.Command.Door.Lock so we can verify
    // it is NOT overwritten by the rejected command.
    let mut val_client = raw_val_client().await;
    let marker = r#"{"marker":"pre-test-value"}"#;
    set_string_signal(&mut val_client, "Vehicle.Command.Door.Lock", marker).await;

    // Subscribe to command_responses on NATS (to verify no response is
    // published for the rejected command).
    let raw_nats = async_nats::connect(NATS_URL).await.unwrap();
    let mut response_sub = raw_nats
        .subscribe(format!("vehicles.{}.command_responses", vin))
        .await
        .unwrap();

    // Spawn command processing loop.
    let cmd_nats = Arc::clone(&nats);
    let cmd_broker = Arc::clone(&broker);
    let token = config.bearer_token.clone();
    tokio::spawn(command_processing_loop(cmd_nats, cmd_broker, token));

    // Allow the command loop to establish its NATS subscription.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Publish a command with an INVALID bearer token.
    let mut headers = async_nats::HeaderMap::new();
    headers.insert("Authorization", "Bearer wrong-token");

    let payload = r#"{"command_id":"cmd-2","action":"lock","doors":["driver"]}"#;
    raw_nats
        .publish_with_headers(
            format!("vehicles.{}.commands", vin),
            headers,
            payload.into(),
        )
        .await
        .unwrap();
    raw_nats.flush().await.unwrap();

    // Wait long enough for any (incorrect) processing to complete.
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify Vehicle.Command.Door.Lock was NOT updated.
    let stored = get_string_signal(&mut val_client, "Vehicle.Command.Door.Lock").await;
    assert_eq!(
        stored.as_deref(),
        Some(marker),
        "Vehicle.Command.Door.Lock should still contain the marker (command must be rejected)"
    );

    // Verify no message was published to command_responses.
    let no_message = timeout(Duration::from_millis(500), response_sub.next()).await;
    assert!(
        no_message.is_err() || no_message.unwrap().is_none(),
        "no message should be published to command_responses for a rejected command"
    );
}
