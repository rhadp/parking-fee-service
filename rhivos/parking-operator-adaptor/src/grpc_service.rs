//! gRPC service handlers for the PARKING_OPERATOR_ADAPTOR.
//!
//! [`ParkingService`] implements the four RPCs defined in the proto spec:
//! `StartSession`, `StopSession`, `GetStatus`, `GetRate`.
//!
//! Requirements: 08-REQ-3.1, 08-REQ-3.2, 08-REQ-3.E1, 08-REQ-3.E2,
//!               08-REQ-4.1, 08-REQ-4.2, 08-REQ-5.1, 08-REQ-5.2

use std::sync::Arc;

use tokio::sync::Mutex;
use tonic::Status;

use crate::broker::SessionPublisher;
use crate::operator::OperatorApi;
use crate::session::{Rate, SessionManager, SessionStatus};

// ---------------------------------------------------------------------------
// Result types (stand-ins until proto code generation is wired up)
// ---------------------------------------------------------------------------

/// Result returned by `start_session`.
#[derive(Debug)]
pub struct StartResult {
    /// Operator-assigned session identifier.
    pub session_id: String,
    /// Status string (e.g. `"active"`).
    pub status: String,
}

/// Result returned by `stop_session`.
#[derive(Debug)]
pub struct StopResult {
    /// The session that was stopped.
    pub session_id: String,
    /// Status string (e.g. `"stopped"`).
    pub status: String,
    /// Duration in seconds.
    pub duration_seconds: u64,
    /// Total parking fee.
    pub total_amount: f64,
    /// ISO 4217 currency code.
    pub currency: String,
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// gRPC service implementation for `ParkingAdaptor`.
///
/// Holds shared references to the session manager, operator client, and broker
/// publisher.  The heavy lifting is done by the underlying modules; this struct
/// just wires them together and maps errors to gRPC `Status` codes.
///
/// # Stub
/// All handlers return stub errors — task group 5 implements the real logic.
pub struct ParkingService {
    pub session: Arc<Mutex<SessionManager>>,
    pub operator: Arc<dyn OperatorApi>,
    pub publisher: Arc<dyn SessionPublisher>,
    pub vehicle_id: String,
}

impl ParkingService {
    /// Create a new `ParkingService`.
    pub fn new(
        session: Arc<Mutex<SessionManager>>,
        operator: Arc<dyn OperatorApi>,
        publisher: Arc<dyn SessionPublisher>,
        vehicle_id: String,
    ) -> Self {
        Self {
            session,
            operator,
            publisher,
            vehicle_id,
        }
    }

    // -----------------------------------------------------------------------
    // RPC handlers
    // -----------------------------------------------------------------------

    /// `StartSession(zone_id)` — start a parking session.
    ///
    /// Returns `ALREADY_EXISTS` if a session is already active (08-REQ-3.E1).
    ///
    /// # Stub — returns `UNIMPLEMENTED` (task group 5)
    pub async fn start_session(&self, zone_id: &str) -> Result<StartResult, Status> {
        // STUB: task group 5 implements real session start via operator API.
        let _ = zone_id;
        Err(Status::unimplemented("start_session not yet implemented"))
    }

    /// `StopSession()` — stop the current parking session.
    ///
    /// Returns `NOT_FOUND` if no session is active (08-REQ-3.E2).
    ///
    /// # Stub — returns `UNIMPLEMENTED` (task group 5)
    pub async fn stop_session(&self) -> Result<StopResult, Status> {
        // STUB: task group 5 implements real session stop via operator API.
        Err(Status::unimplemented("stop_session not yet implemented"))
    }

    /// `GetStatus()` — return the current session status.
    ///
    /// # Stub — always returns `None` (task group 5)
    pub async fn get_status(&self) -> Option<SessionStatus> {
        let s = self.session.lock().await;
        s.get_status()
    }

    /// `GetRate()` — return the cached rate.
    ///
    /// Returns `NOT_FOUND` if no session is active (08-REQ-5.2).
    ///
    /// # Stub — returns `NOT_FOUND` (task group 5)
    pub async fn get_rate(&self) -> Result<Rate, Status> {
        let s = self.session.lock().await;
        s.get_rate()
            .ok_or_else(|| Status::not_found("no active parking session"))
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{MockBrokerPublisher, MockOperatorClient};

    fn make_service() -> ParkingService {
        let session = Arc::new(Mutex::new(SessionManager::new(Some(
            "zone-demo-1".to_string(),
        ))));
        let operator: Arc<dyn OperatorApi> = Arc::new(MockOperatorClient::new());
        let publisher: Arc<dyn SessionPublisher> = Arc::new(MockBrokerPublisher::new());

        ParkingService::new(session, operator, publisher, "DEMO-VIN-001".to_string())
    }

    // -----------------------------------------------------------------------
    // TS-08-7: Manual StartSession
    // -----------------------------------------------------------------------

    /// TS-08-7: StartSession gRPC call starts a session via operator API.
    #[tokio::test]
    async fn test_manual_start_session() {
        let svc = make_service();

        let result = svc.start_session("zone-demo-1").await;
        assert!(result.is_ok(), "start_session should succeed");

        let s = svc.session.lock().await;
        assert!(s.is_active(), "session should be active after StartSession");
    }

    // -----------------------------------------------------------------------
    // TS-08-8: Manual StopSession
    // -----------------------------------------------------------------------

    /// TS-08-8: StopSession gRPC call stops session regardless of lock state.
    #[tokio::test]
    async fn test_manual_stop_session() {
        let svc = make_service();

        // First start a session
        svc.start_session("zone-demo-1").await.unwrap();

        let result = svc.stop_session().await;
        assert!(result.is_ok(), "stop_session should succeed");

        let s = svc.session.lock().await;
        assert!(!s.is_active(), "session should be inactive after StopSession");
    }

    // -----------------------------------------------------------------------
    // TS-08-E5: StartSession while active → ALREADY_EXISTS
    // -----------------------------------------------------------------------

    /// TS-08-E5: StartSession while active returns ALREADY_EXISTS.
    #[tokio::test]
    async fn test_start_session_while_active() {
        let svc = make_service();

        // Start first session
        svc.start_session("zone-demo-1").await.unwrap();

        // Attempt second start
        let err = svc
            .start_session("zone-demo-1")
            .await
            .expect_err("should return an error when session is active");
        assert_eq!(
            err.code(),
            tonic::Code::AlreadyExists,
            "expected ALREADY_EXISTS, got {:?}",
            err.code()
        );
    }

    // -----------------------------------------------------------------------
    // TS-08-E6: StopSession while no session → NOT_FOUND
    // -----------------------------------------------------------------------

    /// TS-08-E6: StopSession while no session returns NOT_FOUND.
    #[tokio::test]
    async fn test_stop_session_while_no_session() {
        let svc = make_service();

        let err = svc
            .stop_session()
            .await
            .expect_err("should return an error when no session");
        assert_eq!(
            err.code(),
            tonic::Code::NotFound,
            "expected NOT_FOUND, got {:?}",
            err.code()
        );
    }
}
