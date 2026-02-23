//! Integration tests for the PARKING_OPERATOR_ADAPTOR.
//!
//! Tests for task group 3 (TS-04-1 through TS-04-5, TS-04-E1 through TS-04-E3)
//! spin up an in-process mock operator HTTP server and the adaptor gRPC service,
//! then exercise the gRPC interface end-to-end.
//!
//! Tests for task group 4 (TS-04-6 through TS-04-14, TS-04-E4 through TS-04-E7,
//! TS-04-P1 through TS-04-P3) remain `#[ignore]` as they require external
//! infrastructure (DATA_BROKER).
//!
//! Test Spec Coverage:
//! - TS-04-1 through TS-04-14 (acceptance criteria)
//! - TS-04-E1 through TS-04-E7 (edge cases)
//! - TS-04-P1, TS-04-P2, TS-04-P3 (property tests)

use std::net::SocketAddr;

use parking_operator_adaptor::grpc_service::ParkingAdaptorService;
use parking_operator_adaptor::operator_client::OperatorClient;
use parking_operator_adaptor::proto::adaptor::parking_adaptor_client::ParkingAdaptorClient;
use parking_operator_adaptor::proto::adaptor::parking_adaptor_server::ParkingAdaptorServer;
use parking_operator_adaptor::proto::adaptor::{
    GetRateRequest, GetStatusRequest, StartSessionRequest, StopSessionRequest,
};
use parking_operator_adaptor::session_manager::SessionManager;

/// Start a minimal mock operator HTTP server.
///
/// Returns the base URL (e.g. "http://127.0.0.1:<port>") and a
/// `tokio::task::JoinHandle` for cleanup.
async fn start_mock_operator() -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(_) => break,
            };

            tokio::spawn(async move {
                handle_http_connection(stream).await;
            });
        }
    });

    (base_url, handle)
}

/// Handle a single HTTP connection from the mock operator.
///
/// This is a minimal HTTP/1.1 handler that simulates the PARKING_OPERATOR
/// REST API responses required for integration testing.
async fn handle_http_connection(stream: tokio::net::TcpStream) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut stream = stream;
    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    if n == 0 {
        return;
    }
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse the HTTP request line
    let first_line = request.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }
    let method = parts[0];
    let path = parts[1];

    let (status, body) = match (method, path) {
        ("POST", "/parking/start") => {
            // Generate a fixed session ID for test predictability
            let session_id = "test-session-001";
            (
                200,
                format!(
                    r#"{{"session_id":"{}","status":"active"}}"#,
                    session_id
                ),
            )
        }
        ("POST", "/parking/stop") => {
            // Parse session_id from body
            let body_start = request.find("\r\n\r\n").map(|i| i + 4).unwrap_or(n);
            let body_str = &request[body_start..];
            let session_id = extract_json_field(body_str, "session_id")
                .unwrap_or_else(|| "unknown".to_string());

            // Check if session exists (we only know about test-session-001)
            if session_id.starts_with("test-session-") {
                (
                    200,
                    format!(
                        r#"{{"session_id":"{}","fee":0.01,"duration_seconds":1,"currency":"EUR"}}"#,
                        session_id
                    ),
                )
            } else {
                (
                    404,
                    format!(r#"{{"error":"session \"{}\" not found"}}"#, session_id),
                )
            }
        }
        ("GET", p) if p.starts_with("/parking/") && p.ends_with("/status") => {
            // Extract session_id from /parking/{session_id}/status
            let parts: Vec<&str> = p.trim_matches('/').split('/').collect();
            if parts.len() >= 3 {
                let session_id = parts[1];
                if session_id.starts_with("test-session-") {
                    (
                        200,
                        format!(
                            r#"{{"session_id":"{}","active":true,"start_time":1708700000,"current_fee":0.01,"currency":"EUR"}}"#,
                            session_id
                        ),
                    )
                } else {
                    (
                        404,
                        format!(r#"{{"error":"session \"{}\" not found"}}"#, session_id),
                    )
                }
            } else {
                (400, r#"{"error":"bad request"}"#.to_string())
            }
        }
        ("GET", p) if p.starts_with("/rate/") => {
            let zone_id = p.trim_start_matches("/rate/");
            match zone_id {
                "zone-munich-central" => (
                    200,
                    r#"{"rate_per_hour":2.5,"currency":"EUR","zone_name":"Munich Central"}"#
                        .to_string(),
                ),
                "zone-munich-west" => (
                    200,
                    r#"{"rate_per_hour":1.5,"currency":"EUR","zone_name":"Munich West"}"#
                        .to_string(),
                ),
                _ => (
                    404,
                    format!(r#"{{"error":"zone \"{}\" not found"}}"#, zone_id),
                ),
            }
        }
        ("GET", "/health") => (200, r#"{"status":"ok"}"#.to_string()),
        _ => (404, r#"{"error":"not found"}"#.to_string()),
    };

    let response = format!(
        "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes()).await;
}

/// Extract a JSON string field value (simple, non-recursive).
fn extract_json_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!(r#""{}":"#, field);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    if let Some(rest) = rest.strip_prefix('"') {
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    } else {
        None
    }
}

/// Start the adaptor gRPC service in-process with a mock operator.
///
/// Returns (grpc_addr, mock_operator_url, gRPC server handle).
async fn start_adaptor_with_mock(
) -> (SocketAddr, String, tokio::task::JoinHandle<()>) {
    let (mock_url, _mock_handle) = start_mock_operator().await;

    let operator = OperatorClient::new(&mock_url);
    let session_mgr = SessionManager::new();
    let service = ParkingAdaptorService::new(operator, session_mgr);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let grpc_addr = listener.local_addr().unwrap();

    let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

    let server_handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(ParkingAdaptorServer::new(service))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    // Give the server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    (grpc_addr, mock_url, server_handle)
}

/// Start the adaptor gRPC service pointing to an unreachable operator.
///
/// Returns (grpc_addr, gRPC server handle).
async fn start_adaptor_unreachable_operator() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let operator = OperatorClient::new("http://127.0.0.1:19999");
    let session_mgr = SessionManager::new();
    let service = ParkingAdaptorService::new(operator, session_mgr);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let grpc_addr = listener.local_addr().unwrap();

    let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

    let server_handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(ParkingAdaptorServer::new(service))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    (grpc_addr, server_handle)
}

// ==========================================================================
// TS-04-1: PARKING_OPERATOR_ADAPTOR exposes gRPC service
// Requirement: 04-REQ-1.1
// ==========================================================================

#[tokio::test]
async fn test_adaptor_grpc_service() {
    // TS-04-1: Verify the adaptor exposes a gRPC service on a configurable
    // address implementing the ParkingAdaptor service.
    let (grpc_addr, _mock_url, _server) = start_adaptor_with_mock().await;

    let mut client =
        ParkingAdaptorClient::connect(format!("http://{}", grpc_addr))
            .await
            .expect("should connect to gRPC server");

    // Call GetRate to verify the server responds
    let response = client
        .get_rate(GetRateRequest {
            zone_id: "zone-munich-central".into(),
        })
        .await;

    assert!(response.is_ok(), "gRPC server should respond to GetRate");
}

// ==========================================================================
// TS-04-2: StartSession returns session_id and status
// Requirement: 04-REQ-1.2
// ==========================================================================

#[tokio::test]
async fn test_adaptor_start_session() {
    // TS-04-2: Verify StartSession with valid vehicle_id and zone_id
    // returns session_id and status.
    let (grpc_addr, _mock_url, _server) = start_adaptor_with_mock().await;

    let mut client =
        ParkingAdaptorClient::connect(format!("http://{}", grpc_addr))
            .await
            .unwrap();

    let response = client
        .start_session(StartSessionRequest {
            vehicle_id: "VIN12345".into(),
            zone_id: "zone-munich-central".into(),
        })
        .await
        .expect("StartSession should succeed");

    let resp = response.into_inner();
    assert!(!resp.session_id.is_empty(), "session_id should be non-empty");
    assert_eq!(resp.status, "active", "status should be 'active'");
}

// ==========================================================================
// TS-04-3: StopSession returns fee, duration, and currency
// Requirement: 04-REQ-1.3
// ==========================================================================

#[tokio::test]
async fn test_adaptor_stop_session() {
    // TS-04-3: Verify StopSession returns fee, duration, and currency.
    let (grpc_addr, _mock_url, _server) = start_adaptor_with_mock().await;

    let mut client =
        ParkingAdaptorClient::connect(format!("http://{}", grpc_addr))
            .await
            .unwrap();

    // Start a session first
    let start_resp = client
        .start_session(StartSessionRequest {
            vehicle_id: "VIN12345".into(),
            zone_id: "zone-munich-central".into(),
        })
        .await
        .unwrap()
        .into_inner();

    let session_id = start_resp.session_id.clone();

    // Wait a moment
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Stop the session
    let stop_resp = client
        .stop_session(StopSessionRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("StopSession should succeed")
        .into_inner();

    assert_eq!(stop_resp.session_id, session_id, "session_id should match");
    assert!(stop_resp.total_fee >= 0.0, "total_fee should be >= 0");
    assert!(
        stop_resp.duration_seconds >= 0,
        "duration_seconds should be >= 0"
    );
    assert!(!stop_resp.currency.is_empty(), "currency should be non-empty");
}

// ==========================================================================
// TS-04-4: GetStatus returns current session state
// Requirement: 04-REQ-1.4
// ==========================================================================

#[tokio::test]
async fn test_adaptor_get_status() {
    // TS-04-4: Verify GetStatus returns current session state.
    let (grpc_addr, _mock_url, _server) = start_adaptor_with_mock().await;

    let mut client =
        ParkingAdaptorClient::connect(format!("http://{}", grpc_addr))
            .await
            .unwrap();

    // Start a session first
    let start_resp = client
        .start_session(StartSessionRequest {
            vehicle_id: "VIN12345".into(),
            zone_id: "zone-munich-central".into(),
        })
        .await
        .unwrap()
        .into_inner();

    let session_id = start_resp.session_id;

    // Get status
    let status_resp = client
        .get_status(GetStatusRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("GetStatus should succeed")
        .into_inner();

    assert_eq!(status_resp.session_id, session_id, "session_id should match");
    assert!(status_resp.active, "session should be active");
    assert!(status_resp.start_time > 0, "start_time should be > 0");
    assert!(
        status_resp.current_fee >= 0.0,
        "current_fee should be >= 0"
    );
    assert!(!status_resp.currency.is_empty(), "currency should be non-empty");
}

// ==========================================================================
// TS-04-5: GetRate returns rate information
// Requirement: 04-REQ-1.5
// ==========================================================================

#[tokio::test]
async fn test_adaptor_get_rate() {
    // TS-04-5: Verify GetRate returns rate, currency, and zone_name.
    let (grpc_addr, _mock_url, _server) = start_adaptor_with_mock().await;

    let mut client =
        ParkingAdaptorClient::connect(format!("http://{}", grpc_addr))
            .await
            .unwrap();

    let rate_resp = client
        .get_rate(GetRateRequest {
            zone_id: "zone-munich-central".into(),
        })
        .await
        .expect("GetRate should succeed")
        .into_inner();

    assert!(
        (rate_resp.rate_per_hour - 2.50).abs() < 0.01,
        "rate_per_hour should be 2.50, got {}",
        rate_resp.rate_per_hour
    );
    assert_eq!(rate_resp.currency, "EUR", "currency should be EUR");
    assert_eq!(
        rate_resp.zone_name, "Munich Central",
        "zone_name should be Munich Central"
    );
}

// ==========================================================================
// TS-04-6: Lock event triggers autonomous session start
// Requirement: 04-REQ-2.1
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor + mock operator + DATA_BROKER running"]
async fn test_autonomous_lock_starts_session() {
    // Publish Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true to DATA_BROKER.
    // Wait for adaptor to process.
    // Assert: mock operator received POST /parking/start.
    todo!("TS-04-6: lock event triggers session start not yet implemented")
}

// ==========================================================================
// TS-04-7: Unlock event triggers autonomous session stop
// Requirement: 04-REQ-2.2
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor + mock operator + DATA_BROKER running"]
async fn test_autonomous_unlock_stops_session() {
    // Publish lock event (start session), wait, then publish unlock event.
    // Assert: mock operator received POST /parking/stop.
    todo!("TS-04-7: unlock event triggers session stop not yet implemented")
}

// ==========================================================================
// TS-04-8: Autonomous start writes SessionActive true
// Requirement: 04-REQ-2.3
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor + mock operator + DATA_BROKER running"]
async fn test_autonomous_start_writes_session_active() {
    // Publish lock event.
    // Wait for adaptor to process.
    // Assert: Vehicle.Parking.SessionActive in DATA_BROKER is true.
    todo!("TS-04-8: autonomous start writes SessionActive not yet implemented")
}

// ==========================================================================
// TS-04-9: Autonomous stop writes SessionActive false
// Requirement: 04-REQ-2.4
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor + mock operator + DATA_BROKER running"]
async fn test_autonomous_stop_writes_session_active() {
    // Start session via lock event, then stop via unlock event.
    // Assert: Vehicle.Parking.SessionActive in DATA_BROKER is false.
    todo!("TS-04-9: autonomous stop writes SessionActive not yet implemented")
}

// ==========================================================================
// TS-04-10: gRPC override updates SessionActive
// Requirement: 04-REQ-2.5
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor + mock operator + DATA_BROKER running"]
async fn test_autonomous_override_updates_session_active() {
    // Call StartSession via gRPC. Assert SessionActive == true.
    // Call StopSession via gRPC. Assert SessionActive == false.
    todo!("TS-04-10: gRPC override updates SessionActive not yet implemented")
}

// ==========================================================================
// TS-04-11: DATA_BROKER connection via network gRPC
// Requirement: 04-REQ-3.1
// ==========================================================================

#[tokio::test]
#[ignore = "requires DATA_BROKER running"]
async fn test_databroker_connection() {
    // Start adaptor with DATABROKER_ADDR=localhost:55556.
    // Wait 3s.
    // Assert: adaptor logs do NOT contain "connection refused" or "connection error".
    todo!("TS-04-11: DATA_BROKER connection test not yet implemented")
}

// ==========================================================================
// TS-04-12: Subscribe to IsLocked events
// Requirement: 04-REQ-3.2
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor + mock operator + DATA_BROKER running"]
async fn test_databroker_subscribe_is_locked() {
    // Publish IsLocked=true to DATA_BROKER.
    // Assert: adaptor reacts (mock operator receives POST /parking/start).
    todo!("TS-04-12: subscribe to IsLocked events not yet implemented")
}

// ==========================================================================
// TS-04-13: Read location from DATA_BROKER
// Requirement: 04-REQ-3.3
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor + DATA_BROKER running"]
async fn test_databroker_read_location() {
    // Set latitude=48.1351, longitude=11.5820 in DATA_BROKER.
    // Trigger a lock event.
    // Assert: adaptor logs contain "latitude" or "location".
    todo!("TS-04-13: read location from DATA_BROKER not yet implemented")
}

// ==========================================================================
// TS-04-14: Write SessionActive to DATA_BROKER
// Requirement: 04-REQ-3.4
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor + mock operator + DATA_BROKER running"]
async fn test_databroker_write_session_active() {
    // Trigger lock event to start session.
    // Assert: Vehicle.Parking.SessionActive readable from DATA_BROKER as true.
    todo!("TS-04-14: write SessionActive to DATA_BROKER not yet implemented")
}

// ==========================================================================
// TS-04-E1: StartSession while session already active
// Requirement: 04-REQ-1.E1
// ==========================================================================

#[tokio::test]
async fn test_edge_start_session_already_active() {
    // TS-04-E1: Verify StartSession while session active returns ALREADY_EXISTS.
    let (grpc_addr, _mock_url, _server) = start_adaptor_with_mock().await;

    let mut client =
        ParkingAdaptorClient::connect(format!("http://{}", grpc_addr))
            .await
            .unwrap();

    // Start first session
    let _first = client
        .start_session(StartSessionRequest {
            vehicle_id: "VIN12345".into(),
            zone_id: "zone-munich-central".into(),
        })
        .await
        .expect("first StartSession should succeed");

    // Try to start a second session
    let result = client
        .start_session(StartSessionRequest {
            vehicle_id: "VIN12345".into(),
            zone_id: "zone-munich-central".into(),
        })
        .await;

    assert!(result.is_err(), "second StartSession should fail");
    let status = result.unwrap_err();
    assert_eq!(
        status.code(),
        tonic::Code::AlreadyExists,
        "should return ALREADY_EXISTS, got {:?}",
        status.code()
    );
}

// ==========================================================================
// TS-04-E2: StopSession with unknown session_id
// Requirement: 04-REQ-1.E2
// ==========================================================================

#[tokio::test]
async fn test_edge_stop_session_unknown() {
    // TS-04-E2: Verify StopSession with unknown session_id returns NOT_FOUND.
    let (grpc_addr, _mock_url, _server) = start_adaptor_with_mock().await;

    let mut client =
        ParkingAdaptorClient::connect(format!("http://{}", grpc_addr))
            .await
            .unwrap();

    let result = client
        .stop_session(StopSessionRequest {
            session_id: "nonexistent-session-id".into(),
        })
        .await;

    assert!(result.is_err(), "StopSession with unknown id should fail");
    let status = result.unwrap_err();
    assert_eq!(
        status.code(),
        tonic::Code::NotFound,
        "should return NOT_FOUND, got {:?}",
        status.code()
    );
}

// ==========================================================================
// TS-04-E3: StartSession when PARKING_OPERATOR unreachable
// Requirement: 04-REQ-1.E3
// ==========================================================================

#[tokio::test]
async fn test_edge_start_session_operator_unreachable() {
    // TS-04-E3: Verify StartSession when operator is unreachable returns UNAVAILABLE.
    let (grpc_addr, _server) = start_adaptor_unreachable_operator().await;

    let mut client =
        ParkingAdaptorClient::connect(format!("http://{}", grpc_addr))
            .await
            .unwrap();

    let result = client
        .start_session(StartSessionRequest {
            vehicle_id: "VIN12345".into(),
            zone_id: "zone-munich-central".into(),
        })
        .await;

    assert!(
        result.is_err(),
        "StartSession with unreachable operator should fail"
    );
    let status = result.unwrap_err();
    assert_eq!(
        status.code(),
        tonic::Code::Unavailable,
        "should return UNAVAILABLE, got {:?}",
        status.code()
    );
    // Check that the error message mentions the operator being unreachable
    let msg = status.message().to_lowercase();
    assert!(
        msg.contains("unreachable") || msg.contains("connection"),
        "error message should mention unreachable or connection, got: {}",
        status.message()
    );
}

// ==========================================================================
// TS-04-E4: Unlock event with no active session
// Requirement: 04-REQ-2.E1
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor + mock operator + DATA_BROKER running"]
async fn test_edge_unlock_no_session() {
    // Publish unlock event (IsLocked=false) with no active session.
    // Assert: no POST /parking/stop call to mock operator.
    todo!("TS-04-E4: unlock with no session not yet implemented")
}

// ==========================================================================
// TS-04-E5: Autonomous start fails when operator unreachable
// Requirement: 04-REQ-2.E2
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor + DATA_BROKER running, operator unreachable"]
async fn test_edge_autonomous_start_operator_unreachable() {
    // Start adaptor with unreachable operator URL.
    // Publish lock event.
    // Assert: SessionActive remains false/unset.
    // Assert: adaptor logs contain "error" or "unreachable".
    todo!("TS-04-E5: autonomous start with unreachable operator not yet implemented")
}

// ==========================================================================
// TS-04-E6: Lock event while session already active
// Requirement: 04-REQ-2.E3
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor + mock operator + DATA_BROKER running"]
async fn test_edge_lock_while_session_active() {
    // Publish lock event (start session).
    // Publish another lock event.
    // Assert: only one POST /parking/start was made.
    todo!("TS-04-E6: duplicate lock event not yet implemented")
}

// ==========================================================================
// TS-04-E7: DATA_BROKER unreachable at startup with retry
// Requirement: 04-REQ-3.E1
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor running, DATA_BROKER not running"]
async fn test_edge_databroker_unreachable_retry() {
    // Start adaptor with DATABROKER_ADDR pointing to unreachable address.
    // Wait 5s.
    // Assert: adaptor is still running (did not crash).
    // Assert: logs contain "retry" or "reconnect" at least twice.
    todo!("TS-04-E7: DATA_BROKER unreachable retry not yet implemented")
}

// ==========================================================================
// TS-04-P1: Session State Consistency (property test)
// Property: After each lock/unlock event, SessionActive == has_active_session.
// Validates: 04-REQ-2.3, 04-REQ-2.4
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor + mock operator + DATA_BROKER running"]
async fn test_property_session_state_consistency() {
    // For a sequence of random lock/unlock events:
    //   After each event, assert SessionActive in DATA_BROKER matches
    //   whether the mock operator has an active session.
    todo!("TS-04-P1: session state consistency property not yet implemented")
}

// ==========================================================================
// TS-04-P2: Autonomous Idempotency (property test)
// Property: N lock events produce exactly 1 start call; M unlock events
//           produce exactly 1 stop call.
// Validates: 04-REQ-2.E1, 04-REQ-2.E3
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor + mock operator + DATA_BROKER running"]
async fn test_property_autonomous_idempotency() {
    // Send N lock events (N>=2). Assert: exactly 1 POST /parking/start.
    // Send M unlock events (M>=2). Assert: exactly 1 POST /parking/stop.
    todo!("TS-04-P2: autonomous idempotency property not yet implemented")
}

// ==========================================================================
// TS-04-P3: Override Precedence (property test)
// Property: Manual gRPC calls override autonomous behavior regardless of
//           lock state; SessionActive reflects the override result.
// Validates: 04-REQ-2.5
// ==========================================================================

#[tokio::test]
#[ignore = "requires adaptor + mock operator + DATA_BROKER running"]
async fn test_property_override_precedence() {
    // For each lock state (locked, unlocked):
    //   Manual StartSession -> SessionActive == true.
    //   Manual StopSession -> SessionActive == false.
    todo!("TS-04-P3: override precedence property not yet implemented")
}
