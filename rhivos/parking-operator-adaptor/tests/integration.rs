//! Integration tests for the PARKING_OPERATOR_ADAPTOR.
//!
//! Tests for task group 3 (TS-04-1 through TS-04-5, TS-04-E1 through TS-04-E3)
//! spin up an in-process mock operator HTTP server and the adaptor gRPC service,
//! then exercise the gRPC interface end-to-end.
//!
//! Tests for task group 4 (TS-04-6 through TS-04-14, TS-04-E4 through TS-04-E7,
//! TS-04-P1 through TS-04-P3) use an in-process `MockDataBrokerClient` to
//! simulate DATA_BROKER events and verify autonomous session management.
//!
//! Test Spec Coverage:
//! - TS-04-1 through TS-04-14 (acceptance criteria)
//! - TS-04-E1 through TS-04-E7 (edge cases)
//! - TS-04-P1, TS-04-P2, TS-04-P3 (property tests)

use std::net::SocketAddr;
use std::sync::Arc;

use parking_operator_adaptor::databroker_client::{
    signals, DataValue, MockDataBrokerClient,
};
use parking_operator_adaptor::event_handler::EventHandler;
use parking_operator_adaptor::grpc_service::ParkingAdaptorService;
use parking_operator_adaptor::operator_client::OperatorClient;
use parking_operator_adaptor::proto::adaptor::parking_adaptor_client::ParkingAdaptorClient;
use parking_operator_adaptor::proto::adaptor::parking_adaptor_server::ParkingAdaptorServer;
use parking_operator_adaptor::proto::adaptor::{
    GetRateRequest, GetStatusRequest, StartSessionRequest, StopSessionRequest,
};
use parking_operator_adaptor::session_manager::SessionManager;
use tokio::sync::Mutex;

/// A recording mock operator that tracks all HTTP calls made to it.
struct RecordingMockOperator {
    base_url: String,
    calls: Arc<Mutex<Vec<(String, String, String)>>>,
    #[allow(dead_code)]
    handle: tokio::task::JoinHandle<()>,
}

/// Start a minimal mock operator HTTP server that records calls.
///
/// Returns (base_url, calls_record, handle).
async fn start_recording_mock_operator() -> RecordingMockOperator {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let calls: Arc<Mutex<Vec<(String, String, String)>>> =
        Arc::new(Mutex::new(Vec::new()));
    let calls_clone = calls.clone();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(_) => break,
            };

            let calls = calls_clone.clone();
            tokio::spawn(async move {
                let mut stream = stream;
                let mut buf = vec![0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    return;
                }
                let request = String::from_utf8_lossy(&buf[..n]).to_string();
                let first_line = request.lines().next().unwrap_or("");
                let parts: Vec<&str> = first_line.split_whitespace().collect();
                if parts.len() < 2 {
                    return;
                }
                let method = parts[0].to_string();
                let path = parts[1].to_string();

                // Extract body (after \r\n\r\n)
                let body = request
                    .find("\r\n\r\n")
                    .map(|i| request[i + 4..].to_string())
                    .unwrap_or_default();

                {
                    let mut c = calls.lock().await;
                    c.push((method.clone(), path.clone(), body));
                }

                let (status, resp_body) = match (method.as_str(), path.as_str()) {
                    ("POST", "/parking/start") => (
                        200,
                        r#"{"session_id":"test-session-001","status":"active"}"#
                            .to_string(),
                    ),
                    ("POST", "/parking/stop") => (
                        200,
                        r#"{"session_id":"test-session-001","fee":0.01,"duration_seconds":1,"currency":"EUR"}"#
                            .to_string(),
                    ),
                    ("GET", p) if p.starts_with("/parking/") && p.ends_with("/status") => {
                        let parts_split: Vec<&str> =
                            p.trim_matches('/').split('/').collect();
                        if parts_split.len() >= 3 {
                            let session_id = parts_split[1];
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
                                    format!(
                                        r#"{{"error":"session \"{}\" not found"}}"#,
                                        session_id
                                    ),
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
                                format!(
                                    r#"{{"error":"zone \"{}\" not found"}}"#,
                                    zone_id
                                ),
                            ),
                        }
                    }
                    ("GET", "/health") => (200, r#"{"status":"ok"}"#.to_string()),
                    _ => (404, r#"{"error":"not found"}"#.to_string()),
                };

                let response = format!(
                    "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    resp_body.len(),
                    resp_body
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });

    RecordingMockOperator {
        base_url,
        calls,
        handle,
    }
}

/// Start a minimal mock operator HTTP server (no call recording, simpler).
///
/// Returns the base URL (e.g. "http://127.0.0.1:<port>") and a
/// `tokio::task::JoinHandle` for cleanup.
async fn start_mock_operator() -> (String, tokio::task::JoinHandle<()>) {
    let mock = start_recording_mock_operator().await;
    (mock.base_url, mock.handle)
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

/// Context for autonomous session tests with DATA_BROKER integration.
struct AutonomousTestContext {
    /// gRPC address of the adaptor.
    grpc_addr: SocketAddr,
    /// Mock DATA_BROKER client.
    databroker: Arc<MockDataBrokerClient>,
    /// Recording mock operator for verifying REST calls.
    mock_operator: RecordingMockOperator,
    /// Shared session manager.
    session_mgr: Arc<Mutex<SessionManager>>,
    /// gRPC server handle.
    #[allow(dead_code)]
    server_handle: tokio::task::JoinHandle<()>,
    /// Event handler handle.
    #[allow(dead_code)]
    event_handle: tokio::task::JoinHandle<()>,
}

/// Start the adaptor with mock DATA_BROKER and recording mock operator.
///
/// This sets up a full autonomous session test environment:
/// - Mock DATA_BROKER (in-process, no external infra needed)
/// - Recording mock operator (HTTP server on random port)
/// - gRPC adaptor service with DATA_BROKER integration
/// - Background event handler processing lock/unlock events
async fn start_autonomous_adaptor() -> AutonomousTestContext {
    let mock_operator = start_recording_mock_operator().await;

    let databroker = Arc::new(MockDataBrokerClient::new());
    let operator = OperatorClient::new(&mock_operator.base_url);
    let session_mgr = SessionManager::new();

    // Create gRPC service with DATA_BROKER integration
    let service = ParkingAdaptorService::with_databroker(
        operator.clone(),
        session_mgr.clone(),
        databroker.clone(),
    );

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

    // Start event handler in background
    let event_handler = EventHandler::new(
        databroker.clone(),
        operator,
        session_mgr.clone(),
        "VIN12345".to_string(),
    );
    let event_handle = tokio::spawn(async move {
        event_handler.run().await;
    });

    // Give everything time to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    AutonomousTestContext {
        grpc_addr,
        databroker,
        mock_operator,
        session_mgr,
        server_handle,
        event_handle,
    }
}

/// Start the adaptor with mock DATA_BROKER and unreachable operator.
async fn start_autonomous_adaptor_unreachable_operator() -> AutonomousTestContext {
    // Use a completely unreachable port for the operator
    let mock_operator = RecordingMockOperator {
        base_url: "http://127.0.0.1:19999".to_string(),
        calls: Arc::new(Mutex::new(Vec::new())),
        handle: tokio::spawn(async {}),
    };

    let databroker = Arc::new(MockDataBrokerClient::new());
    let operator = OperatorClient::new(&mock_operator.base_url);
    let session_mgr = SessionManager::new();

    let service = ParkingAdaptorService::with_databroker(
        operator.clone(),
        session_mgr.clone(),
        databroker.clone(),
    );

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

    let event_handler = EventHandler::new(
        databroker.clone(),
        operator,
        session_mgr.clone(),
        "VIN12345".to_string(),
    );
    let event_handle = tokio::spawn(async move {
        event_handler.run().await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    AutonomousTestContext {
        grpc_addr,
        databroker,
        mock_operator,
        session_mgr,
        server_handle,
        event_handle,
    }
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
async fn test_autonomous_lock_starts_session() {
    // TS-04-6: Verify that receiving a lock event triggers the adaptor to
    // autonomously start a parking session via the PARKING_OPERATOR.
    let ctx = start_autonomous_adaptor().await;

    // Publish lock event via mock DATA_BROKER
    ctx.databroker
        .publish(signals::IS_LOCKED, DataValue::Bool(true))
        .await;

    // Wait for the event to be processed
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify the mock operator received POST /parking/start
    let calls = ctx.mock_operator.calls.lock().await;
    assert!(
        calls.iter().any(|(m, p, _)| m == "POST" && p == "/parking/start"),
        "mock operator should have received POST /parking/start, calls: {:?}",
        *calls
    );
}

// ==========================================================================
// TS-04-7: Unlock event triggers autonomous session stop
// Requirement: 04-REQ-2.2
// ==========================================================================

#[tokio::test]
async fn test_autonomous_unlock_stops_session() {
    // TS-04-7: Verify that receiving an unlock event triggers the adaptor to
    // autonomously stop the active parking session.
    let ctx = start_autonomous_adaptor().await;

    // Start a session via lock event
    ctx.databroker
        .publish(signals::IS_LOCKED, DataValue::Bool(true))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Unlock to stop the session
    ctx.databroker
        .publish(signals::IS_LOCKED, DataValue::Bool(false))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify the mock operator received POST /parking/stop
    let calls = ctx.mock_operator.calls.lock().await;
    assert!(
        calls.iter().any(|(m, p, _)| m == "POST" && p == "/parking/stop"),
        "mock operator should have received POST /parking/stop, calls: {:?}",
        *calls
    );
}

// ==========================================================================
// TS-04-8: Autonomous start writes SessionActive true
// Requirement: 04-REQ-2.3
// ==========================================================================

#[tokio::test]
async fn test_autonomous_start_writes_session_active() {
    // TS-04-8: Verify that after autonomously starting a session, the adaptor
    // writes Vehicle.Parking.SessionActive = true to DATA_BROKER.
    let ctx = start_autonomous_adaptor().await;

    // Publish lock event
    ctx.databroker
        .publish(signals::IS_LOCKED, DataValue::Bool(true))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify SessionActive is true in DATA_BROKER
    let session_active = ctx.databroker.get(signals::SESSION_ACTIVE).await;
    assert!(
        session_active.as_bool(),
        "Vehicle.Parking.SessionActive should be true after lock event"
    );
}

// ==========================================================================
// TS-04-9: Autonomous stop writes SessionActive false
// Requirement: 04-REQ-2.4
// ==========================================================================

#[tokio::test]
async fn test_autonomous_stop_writes_session_active() {
    // TS-04-9: Verify that after autonomously stopping a session, the adaptor
    // writes Vehicle.Parking.SessionActive = false to DATA_BROKER.
    let ctx = start_autonomous_adaptor().await;

    // Start session via lock event
    ctx.databroker
        .publish(signals::IS_LOCKED, DataValue::Bool(true))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Stop session via unlock event
    ctx.databroker
        .publish(signals::IS_LOCKED, DataValue::Bool(false))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify SessionActive is false in DATA_BROKER
    let session_active = ctx.databroker.get(signals::SESSION_ACTIVE).await;
    assert!(
        !session_active.as_bool(),
        "Vehicle.Parking.SessionActive should be false after unlock event"
    );
}

// ==========================================================================
// TS-04-10: gRPC override updates SessionActive
// Requirement: 04-REQ-2.5
// ==========================================================================

#[tokio::test]
async fn test_autonomous_override_updates_session_active() {
    // TS-04-10: Verify that a manual StartSession/StopSession gRPC call
    // overrides autonomous behavior and updates SessionActive.
    let ctx = start_autonomous_adaptor().await;

    let mut client =
        ParkingAdaptorClient::connect(format!("http://{}", ctx.grpc_addr))
            .await
            .unwrap();

    // Manual override: StartSession
    let start_resp = client
        .start_session(StartSessionRequest {
            vehicle_id: "VIN12345".into(),
            zone_id: "zone-munich-central".into(),
        })
        .await
        .expect("manual StartSession should succeed");

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Verify SessionActive == true after StartSession override
    let session_active = ctx.databroker.get(signals::SESSION_ACTIVE).await;
    assert!(
        session_active.as_bool(),
        "SessionActive should be true after manual StartSession"
    );

    // Manual override: StopSession
    let session_id = start_resp.into_inner().session_id;
    client
        .stop_session(StopSessionRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("manual StopSession should succeed");

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Verify SessionActive == false after StopSession override
    let session_active = ctx.databroker.get(signals::SESSION_ACTIVE).await;
    assert!(
        !session_active.as_bool(),
        "SessionActive should be false after manual StopSession"
    );
}

// ==========================================================================
// TS-04-11: DATA_BROKER connection via network gRPC
// Requirement: 04-REQ-3.1
// ==========================================================================

#[tokio::test]
async fn test_databroker_connection() {
    // TS-04-11: Verify the PARKING_OPERATOR_ADAPTOR connects to DATA_BROKER
    // using network gRPC (TCP) at a configurable address.
    //
    // We test this by verifying that the MockDataBrokerClient can be used
    // as a DataBrokerClient (trait-based connection abstraction) and that
    // the KuksaDataBrokerClient properly stores its configured address.

    use parking_operator_adaptor::databroker_client::KuksaDataBrokerClient;

    let client = KuksaDataBrokerClient::new("localhost:55556");
    assert_eq!(client.addr(), "localhost:55556");

    // Verify the mock client works as an in-process alternative
    let mock = MockDataBrokerClient::new();
    use parking_operator_adaptor::databroker_client::DataBrokerClient;
    let result = mock.read(signals::SESSION_ACTIVE).await;
    assert!(result.is_ok(), "mock DATA_BROKER read should succeed");
}

// ==========================================================================
// TS-04-12: Subscribe to IsLocked events
// Requirement: 04-REQ-3.2
// ==========================================================================

#[tokio::test]
async fn test_databroker_subscribe_is_locked() {
    // TS-04-12: Verify the PARKING_OPERATOR_ADAPTOR subscribes to
    // Vehicle.Cabin.Door.Row1.DriverSide.IsLocked events.
    //
    // We verify this by publishing an IsLocked event via MockDataBrokerClient
    // and observing that the event handler reacts (starts a session).
    let ctx = start_autonomous_adaptor().await;

    // Publish IsLocked=true
    ctx.databroker
        .publish(signals::IS_LOCKED, DataValue::Bool(true))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify adaptor reacted by calling the operator
    let calls = ctx.mock_operator.calls.lock().await;
    assert!(
        calls.iter().any(|(m, p, _)| m == "POST" && p == "/parking/start"),
        "adaptor should have started a session via POST /parking/start"
    );
}

// ==========================================================================
// TS-04-13: Read location from DATA_BROKER
// Requirement: 04-REQ-3.3
// ==========================================================================

#[tokio::test]
async fn test_databroker_read_location() {
    // TS-04-13: Verify the PARKING_OPERATOR_ADAPTOR reads
    // Vehicle.CurrentLocation.Latitude and Longitude from DATA_BROKER.
    //
    // We set location values in MockDataBrokerClient, trigger a lock event,
    // and verify the adaptor reads the location (session starts successfully).
    let ctx = start_autonomous_adaptor().await;

    // Set location in DATA_BROKER
    ctx.databroker
        .publish(signals::LATITUDE, DataValue::Float(48.1351))
        .await;
    ctx.databroker
        .publish(signals::LONGITUDE, DataValue::Float(11.5820))
        .await;

    // Wait for events to be delivered then trigger lock
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    ctx.databroker
        .publish(signals::IS_LOCKED, DataValue::Bool(true))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify session was started (location was read successfully)
    let mgr = ctx.session_mgr.lock().await;
    assert!(
        mgr.has_active_session(),
        "session should be active after lock event with location data"
    );
}

// ==========================================================================
// TS-04-14: Write SessionActive to DATA_BROKER
// Requirement: 04-REQ-3.4
// ==========================================================================

#[tokio::test]
async fn test_databroker_write_session_active() {
    // TS-04-14: Verify the PARKING_OPERATOR_ADAPTOR writes
    // Vehicle.Parking.SessionActive to DATA_BROKER.
    let ctx = start_autonomous_adaptor().await;

    // Trigger lock event to start session
    ctx.databroker
        .publish(signals::IS_LOCKED, DataValue::Bool(true))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify SessionActive is readable from DATA_BROKER as true
    let value = ctx.databroker.get(signals::SESSION_ACTIVE).await;
    assert!(
        value.as_bool(),
        "Vehicle.Parking.SessionActive should be true"
    );
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
async fn test_edge_unlock_no_session() {
    // TS-04-E4: Verify that an unlock event is ignored when no session is
    // currently active.
    let ctx = start_autonomous_adaptor().await;

    // Get initial call count
    let initial_count = {
        let calls = ctx.mock_operator.calls.lock().await;
        calls
            .iter()
            .filter(|(m, p, _)| m == "POST" && p == "/parking/stop")
            .count()
    };

    // Publish unlock event with no active session
    ctx.databroker
        .publish(signals::IS_LOCKED, DataValue::Bool(false))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify no POST /parking/stop was made
    let final_count = {
        let calls = ctx.mock_operator.calls.lock().await;
        calls
            .iter()
            .filter(|(m, p, _)| m == "POST" && p == "/parking/stop")
            .count()
    };

    assert_eq!(
        initial_count, final_count,
        "no stop call should be made when no session is active"
    );
}

// ==========================================================================
// TS-04-E5: Autonomous start fails when operator unreachable
// Requirement: 04-REQ-2.E2
// ==========================================================================

#[tokio::test]
async fn test_edge_autonomous_start_operator_unreachable() {
    // TS-04-E5: Verify that when the PARKING_OPERATOR is unreachable during
    // autonomous session start, the adaptor does NOT write SessionActive.
    let ctx = start_autonomous_adaptor_unreachable_operator().await;

    // Publish lock event
    ctx.databroker
        .publish(signals::IS_LOCKED, DataValue::Bool(true))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    // Verify SessionActive remains false/unset
    let session_active = ctx.databroker.get(signals::SESSION_ACTIVE).await;
    assert!(
        !session_active.as_bool(),
        "SessionActive should remain false when operator is unreachable"
    );

    // Verify no session was registered
    let mgr = ctx.session_mgr.lock().await;
    assert!(
        !mgr.has_active_session(),
        "no session should be active when operator is unreachable"
    );
}

// ==========================================================================
// TS-04-E6: Lock event while session already active
// Requirement: 04-REQ-2.E3
// ==========================================================================

#[tokio::test]
async fn test_edge_lock_while_session_active() {
    // TS-04-E6: Verify that a lock event is ignored when a session is already
    // active (no duplicate session started).
    let ctx = start_autonomous_adaptor().await;

    // Start a session via lock event
    ctx.databroker
        .publish(signals::IS_LOCKED, DataValue::Bool(true))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Send another lock event
    ctx.databroker
        .publish(signals::IS_LOCKED, DataValue::Bool(true))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify only one POST /parking/start was made
    let calls = ctx.mock_operator.calls.lock().await;
    let start_count = calls
        .iter()
        .filter(|(m, p, _)| m == "POST" && p == "/parking/start")
        .count();
    assert_eq!(
        start_count, 1,
        "only one start call should be made, got {}",
        start_count
    );
}

// ==========================================================================
// TS-04-E7: DATA_BROKER unreachable at startup with retry
// Requirement: 04-REQ-3.E1
// ==========================================================================

#[tokio::test]
async fn test_edge_databroker_unreachable_retry() {
    // TS-04-E7: Verify that when DATA_BROKER is unreachable at startup, the
    // adaptor retries the connection with exponential backoff.
    //
    // We test this by creating a KuksaDataBrokerClient pointing to an
    // unreachable address and calling connect_with_retry in a background
    // task. We verify it doesn't crash and keeps retrying.

    use parking_operator_adaptor::databroker_client::KuksaDataBrokerClient;

    let client = Arc::new(KuksaDataBrokerClient::new("localhost:19999"));

    let client_clone = client.clone();
    let retry_handle = tokio::spawn(async move {
        client_clone.connect_with_retry().await;
    });

    // Wait for a few retry attempts (they're fast since connection fails quickly)
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // The task should still be running (retrying), not crashed
    assert!(
        !retry_handle.is_finished(),
        "connect_with_retry should keep running (retrying)"
    );

    // Abort the retry task
    retry_handle.abort();

    // Verify the client still reports as not connected
    use parking_operator_adaptor::databroker_client::DataBrokerClient;
    let result = client.subscribe(signals::IS_LOCKED).await;
    assert!(
        result.is_err(),
        "subscribe should fail when not connected"
    );
}

// ==========================================================================
// TS-04-P1: Session State Consistency (property test)
// Property: After each lock/unlock event, SessionActive == has_active_session.
// Validates: 04-REQ-2.3, 04-REQ-2.4
// ==========================================================================

#[tokio::test]
async fn test_property_session_state_consistency() {
    // TS-04-P1: For a sequence of lock/unlock events, SessionActive in
    // DATA_BROKER always matches whether the adaptor has an active session.
    let ctx = start_autonomous_adaptor().await;

    // Sequence: lock, lock, unlock, unlock, lock, unlock
    let events = [true, true, false, false, true, false];

    for (i, &is_locked) in events.iter().enumerate() {
        ctx.databroker
            .publish(signals::IS_LOCKED, DataValue::Bool(is_locked))
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let session_active = ctx.databroker.get(signals::SESSION_ACTIVE).await;
        let has_session = {
            let mgr = ctx.session_mgr.lock().await;
            mgr.has_active_session()
        };

        assert_eq!(
            session_active.as_bool(),
            has_session,
            "event {}: SessionActive ({}) should match has_active_session ({})",
            i,
            session_active.as_bool(),
            has_session
        );
    }
}

// ==========================================================================
// TS-04-P2: Autonomous Idempotency (property test)
// Property: N lock events produce exactly 1 start call; M unlock events
//           produce exactly 1 stop call.
// Validates: 04-REQ-2.E1, 04-REQ-2.E3
// ==========================================================================

#[tokio::test]
async fn test_property_autonomous_idempotency() {
    // TS-04-P2: Sending N lock events (N>=2) should produce exactly 1 start
    // call. Sending M unlock events (M>=2) should produce exactly 1 stop call.
    let ctx = start_autonomous_adaptor().await;

    // Send 3 lock events
    for _ in 0..3 {
        ctx.databroker
            .publish(signals::IS_LOCKED, DataValue::Bool(true))
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    // Verify exactly 1 start call
    let start_count = {
        let calls = ctx.mock_operator.calls.lock().await;
        calls
            .iter()
            .filter(|(m, p, _)| m == "POST" && p == "/parking/start")
            .count()
    };
    assert_eq!(
        start_count, 1,
        "exactly 1 POST /parking/start should be made, got {}",
        start_count
    );

    // Send 3 unlock events
    for _ in 0..3 {
        ctx.databroker
            .publish(signals::IS_LOCKED, DataValue::Bool(false))
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    // Verify exactly 1 stop call
    let stop_count = {
        let calls = ctx.mock_operator.calls.lock().await;
        calls
            .iter()
            .filter(|(m, p, _)| m == "POST" && p == "/parking/stop")
            .count()
    };
    assert_eq!(
        stop_count, 1,
        "exactly 1 POST /parking/stop should be made, got {}",
        stop_count
    );
}

// ==========================================================================
// TS-04-P3: Override Precedence (property test)
// Property: Manual gRPC calls override autonomous behavior regardless of
//           lock state; SessionActive reflects the override result.
// Validates: 04-REQ-2.5
// ==========================================================================

#[tokio::test]
async fn test_property_override_precedence() {
    // TS-04-P3: For each lock state (locked, unlocked), a manual
    // StartSession/StopSession succeeds and SessionActive reflects the result.
    for &lock_state in &[true, false] {
        let ctx = start_autonomous_adaptor().await;

        let mut client =
            ParkingAdaptorClient::connect(format!("http://{}", ctx.grpc_addr))
                .await
                .unwrap();

        // Set lock state (may trigger autonomous session if true)
        ctx.databroker
            .publish(signals::IS_LOCKED, DataValue::Bool(lock_state))
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // If locked, an autonomous session may have started. Stop it first.
        if lock_state {
            let mgr = ctx.session_mgr.lock().await;
            if let Some(id) = mgr.current_session_id() {
                let id = id.to_string();
                drop(mgr);
                let _ = client
                    .stop_session(StopSessionRequest {
                        session_id: id,
                    })
                    .await;
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }

        // Manual override: start session
        let start_resp = client
            .start_session(StartSessionRequest {
                vehicle_id: "VIN12345".into(),
                zone_id: "zone-munich-central".into(),
            })
            .await
            .expect("manual StartSession should succeed regardless of lock state");

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let session_active = ctx.databroker.get(signals::SESSION_ACTIVE).await;
        assert!(
            session_active.as_bool(),
            "SessionActive should be true after manual StartSession (lock_state={})",
            lock_state
        );

        // Manual override: stop session
        let session_id = start_resp.into_inner().session_id;
        client
            .stop_session(StopSessionRequest {
                session_id: session_id.clone(),
            })
            .await
            .expect("manual StopSession should succeed");

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let session_active = ctx.databroker.get(signals::SESSION_ACTIVE).await;
        assert!(
            !session_active.as_bool(),
            "SessionActive should be false after manual StopSession (lock_state={})",
            lock_state
        );
    }
}
