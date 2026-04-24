use crate::broker::{DataBrokerClient, SIGNAL_SESSION_ACTIVE};
use crate::operator::ParkingOperator;
use crate::session::{Rate, Session, SessionState};

use tokio::sync::oneshot;

/// Error type for event processing operations.
#[derive(Debug)]
pub enum EventError {
    /// A session is already active (for StartSession).
    AlreadyExists { session_id: String },
    /// No session is active (for StopSession).
    NoActiveSession,
    /// The PARKING_OPERATOR is unavailable after retries.
    OperatorUnavailable(String),
}

impl std::fmt::Display for EventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventError::AlreadyExists { session_id } => {
                write!(f, "session already exists: {session_id}")
            }
            EventError::NoActiveSession => write!(f, "no active session"),
            EventError::OperatorUnavailable(msg) => {
                write!(f, "operator unavailable: {msg}")
            }
        }
    }
}

impl std::error::Error for EventError {}

/// Result type for a successful start session operation.
#[derive(Debug, Clone)]
pub struct StartSessionResult {
    pub session_id: String,
    pub status: String,
    pub rate_type: String,
    pub rate_amount: f64,
    pub rate_currency: String,
    pub zone_id: String,
    pub start_time: i64,
}

/// Result type for a successful stop session operation.
#[derive(Debug, Clone)]
pub struct StopSessionResult {
    pub session_id: String,
    pub status: String,
    pub duration_seconds: u64,
    pub total_amount: f64,
    pub currency: String,
}

/// Handle a lock/unlock event from DATA_BROKER.
///
/// - Lock (is_locked=true): start a session if none is active.
/// - Unlock (is_locked=false): stop the session if one is active.
/// - Idempotent: lock while active or unlock while inactive is a no-op.
pub async fn handle_lock_event<O: ParkingOperator, B: DataBrokerClient>(
    is_locked: bool,
    session: &mut Session,
    operator: &O,
    broker: &B,
    vehicle_id: &str,
    zone_id: &str,
) {
    if is_locked {
        // Lock event → start session if not active
        if session.is_active() {
            tracing::info!("lock event received but session already active, no-op");
            return;
        }
        match operator.start_session(vehicle_id, zone_id).await {
            Ok(resp) => {
                let start_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                session.start(
                    resp.session_id,
                    zone_id.to_string(),
                    start_time,
                    Rate {
                        rate_type: resp.rate.rate_type,
                        amount: resp.rate.amount,
                        currency: resp.rate.currency,
                    },
                );
                if let Err(e) = broker.set_bool(SIGNAL_SESSION_ACTIVE, true).await {
                    tracing::error!(error = %e, "failed to publish SessionActive=true");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to start session with operator");
            }
        }
    } else {
        // Unlock event → stop session if active
        if !session.is_active() {
            tracing::info!("unlock event received but no active session, no-op");
            return;
        }
        let session_id = session.status().unwrap().session_id.clone();
        match operator.stop_session(&session_id).await {
            Ok(_resp) => {
                session.stop();
                if let Err(e) = broker.set_bool(SIGNAL_SESSION_ACTIVE, false).await {
                    tracing::error!(error = %e, "failed to publish SessionActive=false");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to stop session with operator");
            }
        }
    }
}

/// Handle a manual StartSession request from gRPC.
///
/// Returns `Err(EventError::AlreadyExists)` if a session is already active.
/// On success, updates session state and publishes SessionActive=true.
pub async fn handle_start_session<O: ParkingOperator, B: DataBrokerClient>(
    zone_id: &str,
    session: &mut Session,
    operator: &O,
    broker: &B,
    vehicle_id: &str,
) -> Result<StartSessionResult, EventError> {
    if session.is_active() {
        let session_id = session.status().unwrap().session_id.clone();
        return Err(EventError::AlreadyExists { session_id });
    }

    let resp = operator
        .start_session(vehicle_id, zone_id)
        .await
        .map_err(|e| EventError::OperatorUnavailable(e.to_string()))?;

    let start_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let result = StartSessionResult {
        session_id: resp.session_id.clone(),
        status: resp.status.clone(),
        rate_type: resp.rate.rate_type.clone(),
        rate_amount: resp.rate.amount,
        rate_currency: resp.rate.currency.clone(),
        zone_id: zone_id.to_string(),
        start_time,
    };

    session.start(
        resp.session_id,
        zone_id.to_string(),
        start_time,
        Rate {
            rate_type: resp.rate.rate_type,
            amount: resp.rate.amount,
            currency: resp.rate.currency,
        },
    );

    if let Err(e) = broker.set_bool(SIGNAL_SESSION_ACTIVE, true).await {
        tracing::error!(error = %e, "failed to publish SessionActive=true");
    }

    Ok(result)
}

/// Handle a manual StopSession request from gRPC.
///
/// Returns `Err(EventError::NoActiveSession)` if no session is active.
/// On success, clears session state and publishes SessionActive=false.
pub async fn handle_stop_session<O: ParkingOperator, B: DataBrokerClient>(
    session: &mut Session,
    operator: &O,
    broker: &B,
) -> Result<StopSessionResult, EventError> {
    if !session.is_active() {
        return Err(EventError::NoActiveSession);
    }

    let session_id = session.status().unwrap().session_id.clone();

    let resp = operator
        .stop_session(&session_id)
        .await
        .map_err(|e| EventError::OperatorUnavailable(e.to_string()))?;

    session.stop();

    if let Err(e) = broker.set_bool(SIGNAL_SESSION_ACTIVE, false).await {
        tracing::error!(error = %e, "failed to publish SessionActive=false");
    }

    Ok(StopSessionResult {
        session_id: resp.session_id,
        status: resp.status,
        duration_seconds: resp.duration_seconds,
        total_amount: resp.total_amount,
        currency: resp.currency,
    })
}

/// Internal event type for serialized processing.
///
/// Events from both the DATA_BROKER subscription and gRPC handlers are
/// funneled into a single channel and processed sequentially, ensuring
/// no concurrent session state mutations (08-REQ-9.1).
pub enum SessionEvent {
    /// Lock state changed (from DATA_BROKER subscription).
    LockChanged(bool),
    /// Manual StartSession request (from gRPC).
    ManualStart {
        zone_id: String,
        reply: oneshot::Sender<Result<StartSessionResult, EventError>>,
    },
    /// Manual StopSession request (from gRPC).
    ManualStop {
        reply: oneshot::Sender<Result<StopSessionResult, EventError>>,
    },
    /// Query session status (from gRPC GetStatus).
    QueryStatus {
        reply: oneshot::Sender<Option<SessionState>>,
    },
    /// Query rate (from gRPC GetRate).
    QueryRate {
        reply: oneshot::Sender<Option<Rate>>,
    },
}

/// Process a single session event.
///
/// Called from the main event processing loop for each incoming event,
/// ensuring sequential processing (08-REQ-9.1).
pub async fn process_event<O: ParkingOperator, B: DataBrokerClient>(
    event: SessionEvent,
    session: &mut Session,
    operator: &O,
    broker: &B,
    vehicle_id: &str,
    zone_id: &str,
) {
    match event {
        SessionEvent::LockChanged(is_locked) => {
            handle_lock_event(is_locked, session, operator, broker, vehicle_id, zone_id).await;
        }
        SessionEvent::ManualStart {
            zone_id: z,
            reply,
        } => {
            let result = handle_start_session(&z, session, operator, broker, vehicle_id).await;
            let _ = reply.send(result);
        }
        SessionEvent::ManualStop { reply } => {
            let result = handle_stop_session(session, operator, broker).await;
            let _ = reply.send(result);
        }
        SessionEvent::QueryStatus { reply } => {
            let _ = reply.send(session.status().cloned());
        }
        SessionEvent::QueryRate { reply } => {
            let _ = reply.send(session.rate().cloned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::SIGNAL_SESSION_ACTIVE;
    use crate::session::{Rate, Session};
    use crate::testing::{
        make_start_response, make_stop_response, MockBrokerClient, MockOperatorClient,
    };

    // =====================================================================
    // Task 1.4: Event processing and gRPC handler tests
    // =====================================================================

    // TS-08-2: StartSession RPC Returns Session Info
    // Validates: [08-REQ-1.2]
    #[tokio::test]
    async fn test_start_session_returns_info() {
        // GIVEN no active session, operator returns success
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        operator.on_start_return(make_start_response("sess-1"));
        let broker = MockBrokerClient::new();

        // WHEN handle_start_session is called
        let result =
            handle_start_session("zone-a", &mut session, &operator, &broker, "DEMO-VIN-001")
                .await;

        // THEN response contains session info
        let resp = result.expect("should succeed");
        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.rate_type, "per_hour");
        assert!((resp.rate_amount - 2.5).abs() < f64::EPSILON);
        assert_eq!(resp.rate_currency, "EUR");
        assert!(session.is_active());
    }

    // TS-08-3: StopSession RPC Returns Stop Info
    // Validates: [08-REQ-1.3]
    #[tokio::test]
    async fn test_stop_session_returns_info() {
        // GIVEN an active session
        let mut session = Session::new();
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        };
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        let operator = MockOperatorClient::new();
        operator.on_stop_return(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();

        // WHEN handle_stop_session is called
        let result = handle_stop_session(&mut session, &operator, &broker).await;

        // THEN response contains stop info
        let resp = result.expect("should succeed");
        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.duration_seconds, 3600);
        assert!((resp.total_amount - 2.50).abs() < f64::EPSILON);
        assert!(!session.is_active());
    }

    // TS-08-11: Lock Event Starts Session
    // Validates: [08-REQ-3.3]
    #[tokio::test]
    async fn test_lock_event_starts_session() {
        // GIVEN no active session
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        operator.on_start_return(make_start_response("sess-1"));
        let broker = MockBrokerClient::new();

        // WHEN lock event is received (is_locked=true)
        handle_lock_event(
            true,
            &mut session,
            &operator,
            &broker,
            "DEMO-VIN-001",
            "zone-demo-1",
        )
        .await;

        // THEN operator start_session was called with correct args
        let calls = operator.start_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "DEMO-VIN-001");
        assert_eq!(calls[0].1, "zone-demo-1");

        // AND session is now active
        assert!(session.is_active());

        // AND SessionActive was set to true
        assert_eq!(
            broker.last_set_bool(),
            Some((SIGNAL_SESSION_ACTIVE.to_string(), true))
        );
    }

    // TS-08-12: Unlock Event Stops Session
    // Validates: [08-REQ-3.4]
    #[tokio::test]
    async fn test_unlock_event_stops_session() {
        // GIVEN an active session
        let mut session = Session::new();
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        };
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        let operator = MockOperatorClient::new();
        operator.on_stop_return(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();

        // WHEN unlock event is received (is_locked=false)
        handle_lock_event(
            false,
            &mut session,
            &operator,
            &broker,
            "DEMO-VIN-001",
            "zone-demo-1",
        )
        .await;

        // THEN operator stop_session was called
        let calls = operator.stop_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "sess-1");

        // AND session is now inactive
        assert!(!session.is_active());

        // AND SessionActive was set to false
        assert_eq!(
            broker.last_set_bool(),
            Some((SIGNAL_SESSION_ACTIVE.to_string(), false))
        );
    }

    // TS-08-13: SessionActive Set True on Start
    // Validates: [08-REQ-4.1]
    #[tokio::test]
    async fn test_session_active_set_true() {
        // GIVEN no active session
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        operator.on_start_return(make_start_response("sess-1"));
        let broker = MockBrokerClient::new();

        // WHEN session starts (via lock event)
        handle_lock_event(
            true,
            &mut session,
            &operator,
            &broker,
            "DEMO-VIN-001",
            "zone-demo-1",
        )
        .await;

        // THEN broker.set_bool called with SessionActive=true
        let calls = broker.set_bool_calls();
        assert!(calls
            .iter()
            .any(|(s, v)| s == SIGNAL_SESSION_ACTIVE && *v));
    }

    // TS-08-14: SessionActive Set False on Stop
    // Validates: [08-REQ-4.2]
    #[tokio::test]
    async fn test_session_active_set_false() {
        // GIVEN an active session
        let mut session = Session::new();
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        };
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        let operator = MockOperatorClient::new();
        operator.on_stop_return(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();

        // WHEN session stops (via unlock event)
        handle_lock_event(
            false,
            &mut session,
            &operator,
            &broker,
            "DEMO-VIN-001",
            "zone-demo-1",
        )
        .await;

        // THEN broker.set_bool called with SessionActive=false
        let calls = broker.set_bool_calls();
        assert!(calls
            .iter()
            .any(|(s, v)| s == SIGNAL_SESSION_ACTIVE && !v));
    }

    // TS-08-16: Manual StartSession Override
    // Validates: [08-REQ-5.1]
    #[tokio::test]
    async fn test_manual_start_override() {
        // GIVEN no active session, lock state is false (unlocked)
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        operator.on_start_return(make_start_response("sess-1"));
        let broker = MockBrokerClient::new();

        // WHEN manual StartSession is called (regardless of lock state)
        let result =
            handle_start_session("zone-manual", &mut session, &operator, &broker, "DEMO-VIN-001")
                .await;

        // THEN session starts with operator using zone-manual
        let resp = result.expect("should succeed");
        assert!(!resp.session_id.is_empty());
        assert!(session.is_active());
    }

    // TS-08-17: Manual StopSession Override
    // Validates: [08-REQ-5.2]
    #[tokio::test]
    async fn test_manual_stop_override() {
        // GIVEN an active session (lock state is true)
        let mut session = Session::new();
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        };
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        let operator = MockOperatorClient::new();
        operator.on_stop_return(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();

        // WHEN manual StopSession is called (regardless of lock state)
        let result = handle_stop_session(&mut session, &operator, &broker).await;

        // THEN session stops with operator
        let resp = result.expect("should succeed");
        assert_eq!(resp.session_id, "sess-1");
        assert!(!session.is_active());
    }

    // =====================================================================
    // Task 1.5: Edge case and override tests
    // =====================================================================

    // TS-08-E1: StartSession When Already Active
    // Validates: [08-REQ-1.E1]
    #[tokio::test]
    async fn test_start_session_already_active() {
        // GIVEN an active session
        let mut session = Session::new();
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        };
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();

        // WHEN StartSession is called
        let result =
            handle_start_session("zone-b", &mut session, &operator, &broker, "DEMO-VIN-001")
                .await;

        // THEN error is AlreadyExists
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EventError::AlreadyExists { .. }
        ));
    }

    // TS-08-E2: StopSession When No Session Active
    // Validates: [08-REQ-1.E2]
    #[tokio::test]
    async fn test_stop_session_no_active() {
        // GIVEN no active session
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();

        // WHEN StopSession is called
        let result = handle_stop_session(&mut session, &operator, &broker).await;

        // THEN error is NoActiveSession
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            EventError::NoActiveSession
        ));
    }

    // TS-08-E6: Lock Event While Session Active (No-op)
    // Validates: [08-REQ-3.E1]
    #[tokio::test]
    async fn test_lock_event_noop_when_active() {
        // GIVEN an active session
        let mut session = Session::new();
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        };
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();

        // WHEN lock event is received (is_locked=true)
        handle_lock_event(
            true,
            &mut session,
            &operator,
            &broker,
            "DEMO-VIN-001",
            "zone-demo-1",
        )
        .await;

        // THEN operator was NOT called
        assert_eq!(operator.start_call_count(), 0);

        // AND session is still active with same session_id
        assert!(session.is_active());
        assert_eq!(
            session.status().unwrap().session_id,
            "sess-1"
        );
    }

    // TS-08-E7: Unlock Event While No Session (No-op)
    // Validates: [08-REQ-3.E2]
    #[tokio::test]
    async fn test_unlock_event_noop_when_inactive() {
        // GIVEN no active session
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerClient::new();

        // WHEN unlock event is received (is_locked=false)
        handle_lock_event(
            false,
            &mut session,
            &operator,
            &broker,
            "DEMO-VIN-001",
            "zone-demo-1",
        )
        .await;

        // THEN operator was NOT called
        assert_eq!(operator.stop_call_count(), 0);

        // AND session is still inactive
        assert!(!session.is_active());
    }

    // TS-08-E9: SessionActive Publish Failure
    // Validates: [08-REQ-4.E1]
    #[tokio::test]
    async fn test_session_active_publish_failure() {
        // GIVEN broker configured to fail set_bool
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        operator.on_start_return(make_start_response("sess-1"));
        let broker = MockBrokerClient::new();
        broker.fail_set_bool();

        // WHEN a session starts
        handle_lock_event(
            true,
            &mut session,
            &operator,
            &broker,
            "DEMO-VIN-001",
            "zone-demo-1",
        )
        .await;

        // THEN session state is still updated in memory (active=true)
        assert!(session.is_active());

        // AND the service continues to process events (no panic)
        handle_lock_event(
            false,
            &mut session,
            &operator,
            &broker,
            "DEMO-VIN-001",
            "zone-demo-1",
        )
        .await;
        // Should not panic — broker publish failure is logged but not fatal
    }

    // TS-08-E11: Override Resumes Autonomous on Next Cycle
    // Validates: [08-REQ-5.E1], [08-REQ-5.3]
    #[tokio::test]
    async fn test_override_resumes_autonomous() {
        let mut session = Session::new();
        let operator = MockOperatorClient::new();
        operator.on_start_return(make_start_response("sess-1"));
        operator.on_stop_return(make_stop_response("sess-1"));
        let broker = MockBrokerClient::new();

        // Manual start
        let result =
            handle_start_session("zone-a", &mut session, &operator, &broker, "DEMO-VIN-001")
                .await;
        assert!(result.is_ok());
        assert!(session.is_active());

        // Manual stop (override)
        let result = handle_stop_session(&mut session, &operator, &broker).await;
        assert!(result.is_ok());
        assert!(!session.is_active());

        // Reconfigure mock for next start
        operator.on_start_return(make_start_response("sess-2"));

        // Autonomous lock event should start a new session
        handle_lock_event(
            true,
            &mut session,
            &operator,
            &broker,
            "DEMO-VIN-001",
            "zone-demo-1",
        )
        .await;

        // THEN session is active again (autonomous behavior resumed)
        assert!(session.is_active());

        // AND start was called twice total (once manual, once autonomous)
        assert_eq!(operator.start_call_count(), 2);
    }
}
