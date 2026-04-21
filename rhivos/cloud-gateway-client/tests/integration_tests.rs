//! Integration tests for CLOUD_GATEWAY_CLIENT.
//!
//! These tests exercise end-to-end data flows through NATS and DATA_BROKER by
//! starting the service binary as a subprocess. They are marked `#[ignore]` and
//! require running NATS and DATA_BROKER containers.
//!
//! Run with:
//!   cargo test -p cloud-gateway-client -- --ignored
//!
//! Start infrastructure first:
//!   cd deployments && podman-compose up -d

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::StreamExt;

const NATS_URL: &str = "nats://localhost:4222";
const DATABROKER_ADDR: &str = "http://localhost:55556";

// ── Proto types for DATA_BROKER interaction ───────────────────────────────────

mod proto {
    pub mod kuksa {
        pub mod val {
            pub mod v1 {
                tonic::include_proto!("kuksa.val.v1");
            }
        }
    }
}

use proto::kuksa::val::v1::{
    datapoint, val_service_client::ValServiceClient, DataEntry, Datapoint, EntryRequest,
    EntryUpdate, Field, GetRequest, SetRequest,
};

// ── Infrastructure helpers ────────────────────────────────────────────────────

/// Attempt to connect to NATS. Returns `None` if unavailable.
async fn try_nats_connect() -> Option<async_nats::Client> {
    async_nats::connect(NATS_URL).await.ok()
}

/// Attempt to connect to DATA_BROKER. Returns `None` if unavailable.
async fn try_broker_connect() -> Option<ValServiceClient<tonic::transport::Channel>> {
    ValServiceClient::connect(DATABROKER_ADDR.to_string()).await.ok()
}

/// Spawn the cloud-gateway-client binary configured for the given VIN.
///
/// The child process is killed when dropped (via `kill_on_drop(true)`).
fn start_service(vin: &str) -> tokio::process::Child {
    tokio::process::Command::new(env!("CARGO_BIN_EXE_cloud-gateway-client"))
        .arg("serve")
        .env("VIN", vin)
        .env("NATS_URL", NATS_URL)
        .env("DATABROKER_ADDR", DATABROKER_ADDR)
        .env("BEARER_TOKEN", "demo-token")
        .env("RUST_LOG", "off")
        .kill_on_drop(true)
        .spawn()
        .expect("failed to spawn cloud-gateway-client binary")
}

// ── TS-04-10: End-to-end command flow ────────────────────────────────────────

/// TS-04-10: End-to-end command flow.
///
/// Publishes a valid NATS command and verifies that the payload is written
/// verbatim to `Vehicle.Command.Door.Lock` in DATA_BROKER within 2 seconds.
///
/// Validates: [04-REQ-2.3], [04-REQ-5.2], [04-REQ-6.3]
#[tokio::test]
#[ignore = "requires NATS + DATA_BROKER containers"]
async fn ts_04_10_end_to_end_command_flow() {
    let nats = match try_nats_connect().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP ts_04_10: NATS not available at {NATS_URL}");
            return;
        }
    };
    let mut broker = match try_broker_connect().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP ts_04_10: DATA_BROKER not available at {DATABROKER_ADDR}");
            return;
        }
    };

    // Use a unique VIN to isolate this test from parallel runs.
    let vin = "E2E-CMD-VIN";
    let _child = start_service(vin);

    // Allow the service to start and subscribe to all channels.
    tokio::time::sleep(Duration::from_secs(2)).await;

    let command_payload = r#"{"command_id":"cmd-1","action":"lock","doors":["driver"],"source":"companion_app","vin":"E2E-VIN","timestamp":1700000000}"#;
    let mut headers = async_nats::HeaderMap::new();
    headers.insert("Authorization", "Bearer demo-token");

    nats.publish_with_headers(
        format!("vehicles.{vin}.commands"),
        headers,
        command_payload.as_bytes().to_vec().into(),
    )
    .await
    .expect("failed to publish command to NATS");

    nats.flush().await.expect("failed to flush NATS");

    // Poll DATA_BROKER for up to 2 seconds for the signal update.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let mut found = false;

    while tokio::time::Instant::now() < deadline {
        if let Ok(resp) = broker
            .get(GetRequest {
                entries: vec![EntryRequest {
                    path: "Vehicle.Command.Door.Lock".to_string(),
                    fields: vec![Field::Value as i32],
                }],
            })
            .await
        {
            for entry in resp.into_inner().entries {
                if let Some(dp) = entry.value {
                    if let Some(datapoint::Value::String(s)) = dp.value {
                        if s == command_payload {
                            found = true;
                            break;
                        }
                    }
                }
            }
        }

        if found {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    assert!(
        found,
        "Vehicle.Command.Door.Lock was not updated with the command payload within 2 seconds"
    );
}

// ── TS-04-11: End-to-end response relay ──────────────────────────────────────

/// TS-04-11: End-to-end response relay.
///
/// Writes a response value to `Vehicle.Command.Door.Response` in DATA_BROKER
/// and verifies the JSON is published verbatim to
/// `vehicles.{VIN}.command_responses` on NATS within 2 seconds.
///
/// Validates: [04-REQ-7.1], [04-REQ-7.2]
#[tokio::test]
#[ignore = "requires NATS + DATA_BROKER containers"]
async fn ts_04_11_end_to_end_response_relay() {
    let nats = match try_nats_connect().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP ts_04_11: NATS not available at {NATS_URL}");
            return;
        }
    };
    let mut broker = match try_broker_connect().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP ts_04_11: DATA_BROKER not available at {DATABROKER_ADDR}");
            return;
        }
    };

    let vin = "E2E-RESP-VIN";

    // Subscribe to NATS BEFORE starting the service so we don't miss any message.
    let mut sub = nats
        .subscribe(format!("vehicles.{vin}.command_responses"))
        .await
        .expect("failed to subscribe to command_responses");

    let _child = start_service(vin);

    // Allow the service to start and subscribe to DATA_BROKER.
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Use a timestamp-unique command_id to identify this specific response.
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let response_json = format!(
        r#"{{"command_id":"relay-test-{ts}","status":"success","timestamp":{ts}}}"#
    );

    // Write the response to DATA_BROKER — the service will relay it to NATS.
    broker
        .set(SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: "Vehicle.Command.Door.Response".to_string(),
                    value: Some(Datapoint {
                        value: Some(datapoint::Value::String(response_json.clone())),
                    }),
                }),
                fields: vec![Field::Value as i32],
            }],
        })
        .await
        .expect("failed to set Vehicle.Command.Door.Response");

    // Collect messages for up to 2 seconds; look for our specific message.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let mut found = false;

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(200), sub.next()).await {
            Ok(Some(msg)) => {
                let received = std::str::from_utf8(&msg.payload).unwrap_or("");
                if received == response_json {
                    found = true;
                    break;
                }
            }
            _ => break,
        }
    }

    assert!(
        found,
        "verbatim relay of response JSON not received on NATS within 2 seconds"
    );
}

// ── TS-04-12: End-to-end telemetry ───────────────────────────────────────────

/// TS-04-12: End-to-end telemetry on signal change.
///
/// Sets `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` to `true` in DATA_BROKER
/// and verifies that a telemetry message containing `is_locked:true` and the
/// correct VIN is published to `vehicles.{VIN}.telemetry` on NATS within 2 s.
///
/// Validates: [04-REQ-8.1], [04-REQ-8.2]
#[tokio::test]
#[ignore = "requires NATS + DATA_BROKER containers"]
async fn ts_04_12_end_to_end_telemetry_on_signal_change() {
    let nats = match try_nats_connect().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP ts_04_12: NATS not available at {NATS_URL}");
            return;
        }
    };
    let mut broker = match try_broker_connect().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP ts_04_12: DATA_BROKER not available at {DATABROKER_ADDR}");
            return;
        }
    };

    let vin = "E2E-TELEM-VIN";

    // Subscribe to NATS BEFORE starting the service.
    let mut sub = nats
        .subscribe(format!("vehicles.{vin}.telemetry"))
        .await
        .expect("failed to subscribe to telemetry");

    let _child = start_service(vin);
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Set the lock state signal in DATA_BROKER.
    broker
        .set(SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked".to_string(),
                    value: Some(Datapoint {
                        value: Some(datapoint::Value::Bool(true)),
                    }),
                }),
                fields: vec![Field::Value as i32],
            }],
        })
        .await
        .expect("failed to set IsLocked signal");

    // Collect telemetry messages for up to 2 seconds; look for one with is_locked:true.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    let mut found = false;

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(200), sub.next()).await {
            Ok(Some(msg)) => {
                if let Ok(json) =
                    serde_json::from_slice::<serde_json::Value>(&msg.payload)
                {
                    if json["vin"] == vin && json["is_locked"] == true {
                        found = true;
                        break;
                    }
                }
            }
            _ => break,
        }
    }

    assert!(
        found,
        "telemetry JSON with vin={vin} and is_locked:true not received within 2 seconds"
    );
}

// ── TS-04-13: Self-registration on startup ───────────────────────────────────

/// TS-04-13: Self-registration on startup.
///
/// Verifies that the service publishes a registration message to
/// `vehicles.{VIN}.status` within 5 seconds of starting.
///
/// Note: the service requires both NATS and DATA_BROKER to start successfully;
/// if DATA_BROKER is unavailable, the service exits before publishing
/// registration (per REQ-9.2 and our startup sequence in main.rs).
///
/// Validates: [04-REQ-4.1], [04-REQ-4.2]
#[tokio::test]
#[ignore = "requires NATS + DATA_BROKER containers"]
async fn ts_04_13_self_registration_on_startup() {
    let nats = match try_nats_connect().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP ts_04_13: NATS not available at {NATS_URL}");
            return;
        }
    };

    // DATA_BROKER must also be available — the service exits on failure (REQ-9.2).
    if try_broker_connect().await.is_none() {
        eprintln!("SKIP ts_04_13: DATA_BROKER not available at {DATABROKER_ADDR}");
        return;
    }

    let vin = "REG-VIN";

    // Subscribe to the status subject BEFORE starting the service.
    let mut sub = nats
        .subscribe(format!("vehicles.{vin}.status"))
        .await
        .expect("failed to subscribe to status subject");

    let _child = start_service(vin);

    // Wait up to 5 seconds for the registration message.
    let msg = tokio::time::timeout(Duration::from_secs(5), sub.next())
        .await
        .expect("timed out waiting for self-registration message")
        .expect("status subscription ended unexpectedly");

    let json: serde_json::Value = serde_json::from_slice(&msg.payload)
        .expect("registration message must be valid JSON");

    assert_eq!(json["vin"], vin, "vin field must match");
    assert_eq!(json["status"], "online", "status field must be 'online'");
    assert!(
        json["timestamp"].is_number(),
        "timestamp field must be a number"
    );
}

// ── TS-04-14: Command rejected with invalid token ─────────────────────────────

/// TS-04-14: Command rejected with invalid bearer token.
///
/// Publishes a NATS command with a wrong bearer token and verifies that
/// `Vehicle.Command.Door.Lock` in DATA_BROKER is NOT updated and no message
/// is published to `vehicles.{VIN}.command_responses`.
///
/// Validates: [04-REQ-5.E2]
#[tokio::test]
#[ignore = "requires NATS + DATA_BROKER containers"]
async fn ts_04_14_command_rejected_with_invalid_token() {
    let nats = match try_nats_connect().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP ts_04_14: NATS not available at {NATS_URL}");
            return;
        }
    };
    let mut broker = match try_broker_connect().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP ts_04_14: DATA_BROKER not available at {DATABROKER_ADDR}");
            return;
        }
    };

    let vin = "E2E-AUTH-VIN";
    let _child = start_service(vin);
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Subscribe to command_responses to ensure nothing is published.
    let mut resp_sub = nats
        .subscribe(format!("vehicles.{vin}.command_responses"))
        .await
        .expect("failed to subscribe to command_responses");

    // Read the current value of Vehicle.Command.Door.Lock before sending the command.
    let before_value = broker
        .get(GetRequest {
            entries: vec![EntryRequest {
                path: "Vehicle.Command.Door.Lock".to_string(),
                fields: vec![Field::Value as i32],
            }],
        })
        .await
        .ok()
        .and_then(|r| r.into_inner().entries.into_iter().next())
        .and_then(|e| e.value)
        .and_then(|dp| dp.value)
        .and_then(|v| {
            if let datapoint::Value::String(s) = v {
                Some(s)
            } else {
                None
            }
        });

    // Publish a command with the WRONG bearer token.
    let mut headers = async_nats::HeaderMap::new();
    headers.insert("Authorization", "Bearer wrong-token");

    nats.publish_with_headers(
        format!("vehicles.{vin}.commands"),
        headers,
        r#"{"command_id":"cmd-2","action":"lock","doors":["driver"]}"#
            .as_bytes()
            .to_vec()
            .into(),
    )
    .await
    .expect("failed to publish command");

    nats.flush().await.expect("failed to flush NATS");

    // Wait 500 ms to allow any erroneous reaction.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify no message is published to command_responses.
    let msg = tokio::time::timeout(Duration::from_millis(100), resp_sub.next()).await;
    assert!(
        msg.is_err(),
        "no message should be published to command_responses for a rejected command"
    );

    // Verify Vehicle.Command.Door.Lock was NOT updated.
    let after_value = broker
        .get(GetRequest {
            entries: vec![EntryRequest {
                path: "Vehicle.Command.Door.Lock".to_string(),
                fields: vec![Field::Value as i32],
            }],
        })
        .await
        .ok()
        .and_then(|r| r.into_inner().entries.into_iter().next())
        .and_then(|e| e.value)
        .and_then(|dp| dp.value)
        .and_then(|v| {
            if let datapoint::Value::String(s) = v {
                Some(s)
            } else {
                None
            }
        });

    assert_eq!(
        before_value, after_value,
        "Vehicle.Command.Door.Lock must not change when bearer token is invalid"
    );
}

// ── TS-04-SMOKE-1: Service starts with valid configuration ────────────────────

/// TS-04-SMOKE-1: Service starts with valid configuration.
///
/// Verifies that the service starts without error when both NATS and DATA_BROKER
/// are available and a valid VIN is configured.
///
/// Validates: [04-REQ-2.1], [04-REQ-3.1]
#[tokio::test]
#[ignore = "requires NATS + DATA_BROKER containers"]
async fn ts_04_smoke_1_service_starts_with_valid_config() {
    if try_nats_connect().await.is_none() {
        eprintln!("SKIP ts_04_smoke_1: NATS not available at {NATS_URL}");
        return;
    }
    if try_broker_connect().await.is_none() {
        eprintln!("SKIP ts_04_smoke_1: DATA_BROKER not available at {DATABROKER_ADDR}");
        return;
    }

    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_cloud-gateway-client"))
        .arg("serve")
        .env("VIN", "SMOKE-VIN")
        .env("NATS_URL", NATS_URL)
        .env("DATABROKER_ADDR", DATABROKER_ADDR)
        .env("BEARER_TOKEN", "demo-token")
        .env("RUST_LOG", "off")
        .kill_on_drop(true)
        .spawn()
        .expect("failed to spawn cloud-gateway-client binary");

    // Allow time for the full startup sequence to complete.
    tokio::time::sleep(Duration::from_secs(3)).await;

    // The service should still be running (no crash or early exit).
    let status = child.try_wait().expect("failed to check child process status");
    assert!(
        status.is_none(),
        "Service should be running after successful startup, but exited with: {:?}",
        status
    );
}

// ── TS-04-SMOKE-2: Service exits on missing VIN ───────────────────────────────

/// TS-04-SMOKE-2: Service exits with code 1 when `VIN` is not set.
///
/// This test does not require NATS or DATA_BROKER — the process exits during
/// configuration validation before attempting any connections.
///
/// Validates: [04-REQ-1.E1]
#[tokio::test]
#[ignore = "smoke test: spawns the binary; no infrastructure required"]
async fn ts_04_smoke_2_service_exits_on_missing_vin() {
    let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_cloud-gateway-client"))
        .arg("serve")
        .env_remove("VIN")
        // Point at unreachable addresses so accidental connection attempts fail fast.
        .env("NATS_URL", "nats://127.0.0.1:19222")
        .env("DATABROKER_ADDR", "http://127.0.0.1:19556")
        .env("RUST_LOG", "off")
        .output()
        .await
        .expect("failed to execute cloud-gateway-client binary");

    assert!(
        !output.status.success(),
        "Service must exit with non-zero code when VIN is missing"
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "Service must exit with code 1 when VIN is missing"
    );

    let stderr = std::str::from_utf8(&output.stderr).unwrap_or("");
    assert!(
        stderr.contains("VIN"),
        "stderr must mention VIN; got: {stderr}"
    );
}

// ── TS-04-SMOKE-3: Registration message published on startup ──────────────────

/// TS-04-SMOKE-3: Service publishes a registration message to
/// `vehicles.{VIN}.status` within 5 seconds of startup.
///
/// Validates: [04-REQ-4.1]
#[tokio::test]
#[ignore = "requires NATS + DATA_BROKER containers"]
async fn ts_04_smoke_3_service_publishes_registration_on_startup() {
    let nats = match try_nats_connect().await {
        Some(c) => c,
        None => {
            eprintln!("SKIP ts_04_smoke_3: NATS not available at {NATS_URL}");
            return;
        }
    };
    if try_broker_connect().await.is_none() {
        eprintln!("SKIP ts_04_smoke_3: DATA_BROKER not available at {DATABROKER_ADDR}");
        return;
    }

    let vin = "SMOKE-VIN";

    // Subscribe before starting the service so no message is missed.
    let mut sub = nats
        .subscribe(format!("vehicles.{vin}.status"))
        .await
        .expect("failed to subscribe to status subject");

    let _child = start_service(vin);

    // Wait up to 5 seconds for the registration message.
    let msg = tokio::time::timeout(Duration::from_secs(5), sub.next())
        .await
        .expect("timed out waiting for registration message within 5 seconds")
        .expect("status subscription ended unexpectedly");

    let json: serde_json::Value = serde_json::from_slice(&msg.payload)
        .expect("registration message must be valid JSON");

    assert_eq!(json["vin"], vin, "vin field must match");
    assert_eq!(json["status"], "online", "status must be 'online'");
    assert!(
        json["timestamp"].is_number(),
        "timestamp must be present and numeric"
    );
}

// ── TS-04-15: NATS reconnection with exponential backoff ─────────────────────

/// TS-04-15: NATS reconnection with exponential backoff.
///
/// When the NATS server is unreachable, the service retries with delays
/// 1 s, 2 s, 4 s, 8 s (four intervals across five total attempts) and exits
/// with code 1 after exhausting all attempts.
///
/// The cumulative timeline is: attempt at t=0, t≈1s, t≈3s, t≈7s, t≈15s —
/// matching TS-04-15 and documented in docs/errata/04_cloud_gateway_client.md §E1.
///
/// Validates: [04-REQ-2.2], [04-REQ-2.E1]
#[tokio::test]
#[ignore = "slow test (~15 s); requires NATS to be unreachable on port 19222"]
async fn ts_04_15_nats_reconnection_with_exponential_backoff() {
    // Use a port with nothing listening so every connection attempt gets an
    // immediate ECONNREFUSED (no timeout overhead).
    let unreachable_nats = "nats://127.0.0.1:19222";

    let start = std::time::Instant::now();

    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_cloud-gateway-client"))
        .arg("serve")
        .env("VIN", "BACKOFF-VIN")
        .env("NATS_URL", unreachable_nats)
        .env("DATABROKER_ADDR", DATABROKER_ADDR)
        .env("RUST_LOG", "off")
        .kill_on_drop(true)
        .spawn()
        .expect("failed to spawn cloud-gateway-client binary");

    // Service should exit after exhausting all 5 NATS connection attempts.
    // Total backoff: 1s + 2s + 4s + 8s = 15 s. Allow up to 60 s for slow CI.
    let status = tokio::time::timeout(Duration::from_secs(60), child.wait())
        .await
        .expect("service did not exit within 60 seconds")
        .expect("failed to wait for child process");

    let elapsed = start.elapsed();

    // Service must exit with code 1.
    assert!(!status.success(), "Service must exit with non-zero code");
    assert_eq!(status.code(), Some(1), "Service must exit with code 1");

    // Service must have waited at least the sum of all backoff delays (~15 s).
    // Use 12 s as a conservative lower bound to allow for scheduling jitter.
    assert!(
        elapsed >= Duration::from_secs(12),
        "Expected at least ~15 s of exponential backoff, but finished in {:.1}s",
        elapsed.as_secs_f64()
    );
}
