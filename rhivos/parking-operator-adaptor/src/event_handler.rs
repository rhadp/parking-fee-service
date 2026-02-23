//! Autonomous session event handler for the PARKING_OPERATOR_ADAPTOR.
//!
//! Processes lock/unlock events from DATA_BROKER and autonomously starts
//! or stops parking sessions with the PARKING_OPERATOR. This module
//! implements the core logic for Requirements 04-REQ-2.1 through 04-REQ-2.5.

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::databroker_client::{signals, DataBrokerClient, DataValue};
use crate::operator_client::OperatorClient;
use crate::session_manager::SessionManager;

/// Autonomous session event handler.
///
/// Listens to lock/unlock events from the DATA_BROKER and manages
/// parking sessions accordingly. Also handles manual overrides from
/// gRPC calls by coordinating with the `SessionManager`.
pub struct EventHandler<D: DataBrokerClient> {
    /// DATA_BROKER client for subscribing, reading, and writing signals.
    databroker: Arc<D>,
    /// REST client for the PARKING_OPERATOR.
    operator: OperatorClient,
    /// Shared session manager (also used by the gRPC service).
    session_mgr: Arc<Mutex<SessionManager>>,
    /// Vehicle identifier.
    vehicle_id: String,
    /// Default zone_id for autonomous sessions (used when location-based
    /// zone mapping is not available).
    default_zone_id: String,
}

impl<D: DataBrokerClient> EventHandler<D> {
    /// Create a new event handler.
    pub fn new(
        databroker: Arc<D>,
        operator: OperatorClient,
        session_mgr: Arc<Mutex<SessionManager>>,
        vehicle_id: String,
    ) -> Self {
        EventHandler {
            databroker,
            operator,
            session_mgr,
            vehicle_id,
            default_zone_id: "zone-munich-central".to_string(),
        }
    }

    /// Run the event handling loop.
    ///
    /// Subscribes to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` events
    /// and processes them until the subscription is closed or an
    /// unrecoverable error occurs.
    pub async fn run(&self) {
        let mut rx = match self.databroker.subscribe(signals::IS_LOCKED).await {
            Ok(rx) => rx,
            Err(e) => {
                eprintln!("event_handler: failed to subscribe to IsLocked: {}", e);
                return;
            }
        };

        eprintln!("event_handler: subscribed to {}", signals::IS_LOCKED);

        loop {
            match rx.recv().await {
                Ok(event) => {
                    // Only process IsLocked events
                    if event.path != signals::IS_LOCKED {
                        continue;
                    }

                    let is_locked = event.value.as_bool();
                    eprintln!(
                        "event_handler: received IsLocked = {} event",
                        is_locked
                    );

                    if is_locked {
                        self.handle_lock_event().await;
                    } else {
                        self.handle_unlock_event().await;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    eprintln!("event_handler: subscription channel closed");
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!(
                        "event_handler: subscription lagged by {} events",
                        n
                    );
                    // Continue processing
                }
            }
        }
    }

    /// Handle a lock event (IsLocked = true).
    ///
    /// If no session is active, starts a new parking session autonomously:
    /// 1. Read location from DATA_BROKER for zone context
    /// 2. Call POST /parking/start on PARKING_OPERATOR
    /// 3. Write Vehicle.Parking.SessionActive = true to DATA_BROKER
    ///
    /// If a session is already active, the event is ignored (idempotent).
    /// (04-REQ-2.E3)
    async fn handle_lock_event(&self) {
        let has_session = {
            let mgr = self.session_mgr.lock().await;
            mgr.has_active_session()
        };

        if has_session {
            eprintln!(
                "event_handler: ignoring lock event, session already active"
            );
            return;
        }

        // Read location from DATA_BROKER for context
        let latitude = self
            .databroker
            .read(signals::LATITUDE)
            .await
            .unwrap_or(DataValue::NotAvailable);
        let longitude = self
            .databroker
            .read(signals::LONGITUDE)
            .await
            .unwrap_or(DataValue::NotAvailable);

        eprintln!(
            "event_handler: location: latitude={}, longitude={}",
            latitude.as_float(),
            longitude.as_float()
        );

        // Determine zone from location (simplified: use default zone)
        let zone_id = self.default_zone_id.clone();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Call the parking operator to start a session
        match self
            .operator
            .start_session(&self.vehicle_id, &zone_id, timestamp)
            .await
        {
            Ok(response) => {
                eprintln!(
                    "event_handler: autonomous session started: session_id={}",
                    response.session_id
                );

                // Register in session manager
                {
                    let mut mgr = self.session_mgr.lock().await;
                    if let Err(e) = mgr.start_session(
                        response.session_id,
                        zone_id,
                        timestamp,
                        false, // autonomous, not an override
                    ) {
                        eprintln!(
                            "event_handler: failed to register session: {}",
                            e
                        );
                        return;
                    }
                }

                // Write SessionActive = true to DATA_BROKER
                if let Err(e) = self
                    .databroker
                    .write(signals::SESSION_ACTIVE, DataValue::Bool(true))
                    .await
                {
                    eprintln!(
                        "event_handler: failed to write SessionActive: {}",
                        e
                    );
                }
            }
            Err(e) => {
                // 04-REQ-2.E2: Log error, do NOT write SessionActive = true
                eprintln!(
                    "event_handler: error starting autonomous session: {} (operator unreachable)",
                    e
                );
            }
        }
    }

    /// Handle an unlock event (IsLocked = false).
    ///
    /// If a session is active, stops it autonomously:
    /// 1. Call POST /parking/stop on PARKING_OPERATOR
    /// 2. Write Vehicle.Parking.SessionActive = false to DATA_BROKER
    ///
    /// If no session is active, the event is ignored (idempotent).
    /// (04-REQ-2.E1)
    async fn handle_unlock_event(&self) {
        let session_id = {
            let mgr = self.session_mgr.lock().await;
            match mgr.current_session_id() {
                Some(id) => id.to_string(),
                None => {
                    eprintln!(
                        "event_handler: ignoring unlock event, no active session"
                    );
                    return;
                }
            }
        };

        // Call the parking operator to stop the session
        match self.operator.stop_session(&session_id).await {
            Ok(response) => {
                eprintln!(
                    "event_handler: autonomous session stopped: session_id={}, fee={}, duration={}s",
                    response.session_id, response.fee, response.duration_seconds
                );

                // Remove from session manager
                {
                    let mut mgr = self.session_mgr.lock().await;
                    let _ = mgr.stop_session(&session_id);
                }

                // Write SessionActive = false to DATA_BROKER
                if let Err(e) = self
                    .databroker
                    .write(signals::SESSION_ACTIVE, DataValue::Bool(false))
                    .await
                {
                    eprintln!(
                        "event_handler: failed to write SessionActive: {}",
                        e
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "event_handler: error stopping autonomous session: {}",
                    e
                );
            }
        }
    }

    /// Write SessionActive to DATA_BROKER after a manual gRPC override.
    ///
    /// This is called by the gRPC service when StartSession or StopSession
    /// is invoked manually, ensuring DATA_BROKER reflects the override.
    /// (04-REQ-2.5)
    pub async fn write_session_active(&self, active: bool) {
        if let Err(e) = self
            .databroker
            .write(signals::SESSION_ACTIVE, DataValue::Bool(active))
            .await
        {
            eprintln!(
                "event_handler: failed to write SessionActive={}: {}",
                active, e
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::databroker_client::MockDataBrokerClient;

    /// Create a mock operator HTTP server that records calls.
    async fn start_recording_mock_operator(
    ) -> (String, Arc<Mutex<Vec<(String, String)>>>, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let calls: Arc<Mutex<Vec<(String, String)>>> =
            Arc::new(Mutex::new(Vec::new()));
        let calls_clone = calls.clone();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
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

                    {
                        let mut c = calls.lock().await;
                        c.push((method.clone(), path.clone()));
                    }

                    let (status, body) = match (method.as_str(), path.as_str()) {
                        ("POST", "/parking/start") => (
                            200,
                            r#"{"session_id":"auto-session-001","status":"active"}"#
                                .to_string(),
                        ),
                        ("POST", "/parking/stop") => (
                            200,
                            r#"{"session_id":"auto-session-001","fee":0.01,"duration_seconds":1,"currency":"EUR"}"#
                                .to_string(),
                        ),
                        _ => (404, r#"{"error":"not found"}"#.to_string()),
                    };

                    let response = format!(
                        "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        status,
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                });
            }
        });

        (base_url, calls, handle)
    }

    #[tokio::test]
    async fn test_lock_event_starts_session() {
        let (mock_url, calls, _handle) = start_recording_mock_operator().await;
        let databroker = Arc::new(MockDataBrokerClient::new());
        let operator = OperatorClient::new(&mock_url);
        let session_mgr = SessionManager::new();

        let handler = EventHandler::new(
            databroker.clone(),
            operator,
            session_mgr.clone(),
            "VIN12345".to_string(),
        );

        // Simulate a lock event
        handler.handle_lock_event().await;

        // Verify the operator was called
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let recorded = calls.lock().await;
        assert!(
            recorded.iter().any(|(m, p)| m == "POST" && p == "/parking/start"),
            "should have called POST /parking/start, calls: {:?}",
            *recorded
        );

        // Verify session is active in session manager
        let mgr = session_mgr.lock().await;
        assert!(mgr.has_active_session());

        // Verify SessionActive was written
        let session_active = databroker.get(signals::SESSION_ACTIVE).await;
        assert!(session_active.as_bool());
    }

    #[tokio::test]
    async fn test_unlock_event_stops_session() {
        let (mock_url, calls, _handle) = start_recording_mock_operator().await;
        let databroker = Arc::new(MockDataBrokerClient::new());
        let operator = OperatorClient::new(&mock_url);
        let session_mgr = SessionManager::new();

        let handler = EventHandler::new(
            databroker.clone(),
            operator,
            session_mgr.clone(),
            "VIN12345".to_string(),
        );

        // First start a session
        handler.handle_lock_event().await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Then unlock
        handler.handle_unlock_event().await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Verify stop was called
        let recorded = calls.lock().await;
        assert!(
            recorded.iter().any(|(m, p)| m == "POST" && p == "/parking/stop"),
            "should have called POST /parking/stop, calls: {:?}",
            *recorded
        );

        // Verify session is no longer active
        let mgr = session_mgr.lock().await;
        assert!(!mgr.has_active_session());

        // Verify SessionActive was written as false
        let session_active = databroker.get(signals::SESSION_ACTIVE).await;
        assert!(!session_active.as_bool());
    }

    #[tokio::test]
    async fn test_lock_event_ignored_when_session_active() {
        let (mock_url, calls, _handle) = start_recording_mock_operator().await;
        let databroker = Arc::new(MockDataBrokerClient::new());
        let operator = OperatorClient::new(&mock_url);
        let session_mgr = SessionManager::new();

        let handler = EventHandler::new(
            databroker.clone(),
            operator,
            session_mgr.clone(),
            "VIN12345".to_string(),
        );

        // Start a session
        handler.handle_lock_event().await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Send another lock event — should be ignored
        handler.handle_lock_event().await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Verify only one start call
        let recorded = calls.lock().await;
        let start_count = recorded
            .iter()
            .filter(|(m, p)| m == "POST" && p == "/parking/start")
            .count();
        assert_eq!(start_count, 1, "should only have one start call");
    }

    #[tokio::test]
    async fn test_unlock_event_ignored_when_no_session() {
        let (mock_url, calls, _handle) = start_recording_mock_operator().await;
        let databroker = Arc::new(MockDataBrokerClient::new());
        let operator = OperatorClient::new(&mock_url);
        let session_mgr = SessionManager::new();

        let handler = EventHandler::new(
            databroker.clone(),
            operator,
            session_mgr.clone(),
            "VIN12345".to_string(),
        );

        // Send unlock event with no active session
        handler.handle_unlock_event().await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Verify no stop call was made
        let recorded = calls.lock().await;
        let stop_count = recorded
            .iter()
            .filter(|(m, p)| m == "POST" && p == "/parking/stop")
            .count();
        assert_eq!(stop_count, 0, "should not have called stop");
    }

    #[tokio::test]
    async fn test_lock_event_operator_unreachable() {
        let databroker = Arc::new(MockDataBrokerClient::new());
        let operator = OperatorClient::new("http://127.0.0.1:19999");
        let session_mgr = SessionManager::new();

        let handler = EventHandler::new(
            databroker.clone(),
            operator,
            session_mgr.clone(),
            "VIN12345".to_string(),
        );

        // Try to start session — operator is unreachable
        handler.handle_lock_event().await;

        // Session should NOT be active
        let mgr = session_mgr.lock().await;
        assert!(!mgr.has_active_session());

        // SessionActive should NOT be true
        let session_active = databroker.get(signals::SESSION_ACTIVE).await;
        assert!(!session_active.as_bool());
    }

    #[tokio::test]
    async fn test_write_session_active_override() {
        let databroker = Arc::new(MockDataBrokerClient::new());
        let operator = OperatorClient::new("http://127.0.0.1:19999");
        let session_mgr = SessionManager::new();

        let handler = EventHandler::new(
            databroker.clone(),
            operator,
            session_mgr,
            "VIN12345".to_string(),
        );

        handler.write_session_active(true).await;
        let value = databroker.get(signals::SESSION_ACTIVE).await;
        assert!(value.as_bool());

        handler.write_session_active(false).await;
        let value = databroker.get(signals::SESSION_ACTIVE).await;
        assert!(!value.as_bool());
    }

    #[tokio::test]
    async fn test_location_read_on_lock() {
        let (mock_url, _calls, _handle) = start_recording_mock_operator().await;
        let databroker = Arc::new(MockDataBrokerClient::new());
        let operator = OperatorClient::new(&mock_url);
        let session_mgr = SessionManager::new();

        // Set location in databroker
        databroker
            .publish(signals::LATITUDE, DataValue::Float(48.1351))
            .await;
        databroker
            .publish(signals::LONGITUDE, DataValue::Float(11.5820))
            .await;

        let handler = EventHandler::new(
            databroker.clone(),
            operator,
            session_mgr.clone(),
            "VIN12345".to_string(),
        );

        // Trigger a lock event — should read location
        handler.handle_lock_event().await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Session should be active (location was read, session started)
        let mgr = session_mgr.lock().await;
        assert!(mgr.has_active_session());
    }
}
