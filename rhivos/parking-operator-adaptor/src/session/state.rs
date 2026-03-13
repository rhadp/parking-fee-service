use serde::{Deserialize, Serialize};

/// Pricing information for a parking session.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rate {
    /// The rate type: "per_hour" or "flat_fee".
    pub rate_type: String,
    /// The rate amount.
    pub amount: f64,
    /// The currency code (e.g. "EUR").
    pub currency: String,
}

/// Snapshot of an active parking session's state.
///
/// Returned by [`SessionManager::get_status`] and [`SessionManager::stop`].
#[derive(Clone, Debug)]
pub struct SessionInfo {
    pub session_id: String,
    pub zone_id: String,
    pub start_time: i64,
    pub rate: Rate,
    pub active: bool,
}

/// Represents the phase of the parking session state machine.
///
/// Used internally by [`SessionManager`] and exposed for callers that need
/// fine-grained phase information (e.g. the autonomous event loop).
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Idle,
    Starting,
    Active,
    Stopping,
}

/// Manages the parking session state machine.
///
/// Provides two API levels:
/// - **High-level** (`start`, `stop`, `get_status`, `get_rate`, `is_active`):
///   matches the design.md interface for direct session management.
/// - **Low-level** (`try_start`, `confirm_start`, `fail_start`, `try_stop`,
///   `confirm_stop`, `fail_stop`): used by the autonomous event loop and gRPC
///   handlers that separate the transition request from the operator response.
pub struct SessionManager {
    state: SessionState,
    session_id: Option<String>,
    zone_id: Option<String>,
    start_time: Option<i64>,
    rate: Option<Rate>,
}

impl SessionManager {
    /// Create a new SessionManager in the Idle state.
    pub fn new(zone_id: Option<String>) -> Self {
        Self {
            state: SessionState::Idle,
            session_id: None,
            zone_id,
            start_time: None,
            rate: None,
        }
    }

    // -----------------------------------------------------------------------
    // High-level API (design.md interface)
    // -----------------------------------------------------------------------

    /// Start a session directly (stores session_id, zone_id, rate, and
    /// current timestamp). Returns `SessionError::AlreadyActive` if a
    /// session is already in progress.
    pub fn start(
        &mut self,
        session_id: &str,
        zone_id: &str,
        rate: Rate,
    ) -> Result<(), SessionError> {
        if self.state != SessionState::Idle {
            return Err(SessionError::AlreadyActive);
        }
        self.state = SessionState::Active;
        self.session_id = Some(session_id.to_string());
        self.zone_id = Some(zone_id.to_string());
        self.start_time = Some(chrono::Utc::now().timestamp());
        self.rate = Some(rate);
        Ok(())
    }

    /// Stop the current session. Returns the session info snapshot on
    /// success, or `SessionError::NotActive` if no session is active.
    pub fn stop(&mut self) -> Result<SessionInfo, SessionError> {
        if self.state != SessionState::Active {
            return Err(SessionError::NotActive);
        }

        let info = SessionInfo {
            session_id: self.session_id.clone().unwrap_or_default(),
            zone_id: self.zone_id.clone().unwrap_or_default(),
            start_time: self.start_time.unwrap_or(0),
            rate: self.rate.clone().unwrap_or(Rate {
                rate_type: String::new(),
                amount: 0.0,
                currency: String::new(),
            }),
            active: false,
        };

        self.state = SessionState::Idle;
        self.session_id = None;
        self.start_time = None;
        self.rate = None;
        Ok(info)
    }

    /// Return the current session info if a session is active, or `None`.
    pub fn get_status(&self) -> Option<SessionInfo> {
        if self.state == SessionState::Active {
            Some(SessionInfo {
                session_id: self.session_id.clone().unwrap_or_default(),
                zone_id: self.zone_id.clone().unwrap_or_default(),
                start_time: self.start_time.unwrap_or(0),
                rate: self.rate.clone().unwrap_or(Rate {
                    rate_type: String::new(),
                    amount: 0.0,
                    currency: String::new(),
                }),
                active: true,
            })
        } else {
            None
        }
    }

    /// Return the cached rate if a session is active, or `None`.
    pub fn get_rate(&self) -> Option<Rate> {
        if self.state == SessionState::Active {
            self.rate.clone()
        } else {
            None
        }
    }

    /// Whether a session is currently active.
    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }

    // -----------------------------------------------------------------------
    // Low-level API (state machine transitions)
    // -----------------------------------------------------------------------

    /// Returns the current session phase.
    pub fn state(&self) -> &SessionState {
        &self.state
    }

    /// Returns the current session ID, if any.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Returns the configured zone ID, if any.
    pub fn zone_id(&self) -> Option<&str> {
        self.zone_id.as_deref()
    }

    /// Attempt to transition from Idle to Starting.
    /// Returns Ok(()) if transition is valid, Err if session already active.
    pub fn try_start(&mut self) -> Result<(), SessionError> {
        match self.state {
            SessionState::Idle => {
                self.state = SessionState::Starting;
                Ok(())
            }
            _ => Err(SessionError::AlreadyActive),
        }
    }

    /// Confirm a successful session start from the operator.
    /// Transitions from Starting to Active and stores session metadata.
    pub fn confirm_start(&mut self, session_id: String) {
        self.state = SessionState::Active;
        self.session_id = Some(session_id);
        self.start_time = Some(chrono::Utc::now().timestamp());
    }

    /// Confirm a successful session start from the operator, including rate.
    pub fn confirm_start_with_rate(&mut self, session_id: String, rate: Rate) {
        self.state = SessionState::Active;
        self.session_id = Some(session_id);
        self.start_time = Some(chrono::Utc::now().timestamp());
        self.rate = Some(rate);
    }

    /// Handle a failed session start from the operator.
    /// Transitions from Starting back to Idle.
    pub fn fail_start(&mut self) {
        self.state = SessionState::Idle;
        self.session_id = None;
        self.start_time = None;
        self.rate = None;
    }

    /// Attempt to transition from Active to Stopping.
    /// Returns Ok(()) if transition is valid, Err if no session active.
    pub fn try_stop(&mut self) -> Result<(), SessionError> {
        match self.state {
            SessionState::Active => {
                self.state = SessionState::Stopping;
                Ok(())
            }
            _ => Err(SessionError::NoActiveSession),
        }
    }

    /// Confirm a successful session stop from the operator.
    /// Transitions from Stopping to Idle.
    pub fn confirm_stop(&mut self) {
        self.state = SessionState::Idle;
        self.session_id = None;
        self.start_time = None;
        self.rate = None;
    }

    /// Handle a failed session stop from the operator.
    /// Transitions from Stopping back to Active (per 08-REQ-2.E2:
    /// session state is not updated on stop failure).
    pub fn fail_stop(&mut self) {
        self.state = SessionState::Active;
    }
}

/// Errors that can occur during session state transitions.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("session already active")]
    AlreadyActive,
    #[error("no active session")]
    NoActiveSession,
    #[error("no active session")]
    NotActive,
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // High-level API tests (design.md / test_spec.md)
    // -----------------------------------------------------------------------

    /// TS-08-2: After successful start, session_id, zone_id, start_time, and
    /// rate are stored and retrievable via get_status.
    #[test]
    fn test_session_state_stored_after_start() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.50,
            currency: "EUR".to_string(),
        };
        mgr.start("sess-001", "zone-1", rate).unwrap();

        let status = mgr.get_status().expect("should have status");
        assert_eq!(status.session_id, "sess-001");
        assert_eq!(status.zone_id, "zone-1");
        assert_eq!(status.rate.amount, 2.50);
        assert_eq!(status.rate.rate_type, "per_hour");
        assert_eq!(status.rate.currency, "EUR");
        assert!(status.active);
        assert!(status.start_time > 0);
    }

    /// TS-08-5: After stop, session state is cleared.
    #[test]
    fn test_session_cleared_after_stop() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.50,
            currency: "EUR".to_string(),
        };
        mgr.start("sess-001", "zone-1", rate).unwrap();
        assert!(mgr.is_active());

        let info = mgr.stop().unwrap();
        assert!(!info.active);
        assert_eq!(info.session_id, "sess-001");
        assert!(!mgr.is_active());
    }

    /// TS-08-10: GetStatus returns session info when active.
    #[test]
    fn test_get_status_active_session() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.50,
            currency: "EUR".to_string(),
        };
        mgr.start("sess-001", "zone-1", rate).unwrap();

        let status = mgr.get_status().expect("should have status");
        assert_eq!(status.session_id, "sess-001");
        assert!(status.active);
    }

    /// TS-08-11: GetStatus returns None when no session.
    #[test]
    fn test_get_status_no_session() {
        let mgr = SessionManager::new(Some("zone-1".to_string()));
        assert!(mgr.get_status().is_none());
    }

    /// TS-08-12: GetRate returns cached rate when session active.
    #[test]
    fn test_get_rate_active_session() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.50,
            currency: "EUR".to_string(),
        };
        mgr.start("sess-001", "zone-1", rate).unwrap();

        let r = mgr.get_rate().expect("should have rate");
        assert_eq!(r.rate_type, "per_hour");
        assert_eq!(r.amount, 2.50);
        assert_eq!(r.currency, "EUR");
    }

    /// TS-08-E5: start() while active returns AlreadyActive.
    #[test]
    fn test_start_session_while_active() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.50,
            currency: "EUR".to_string(),
        };
        mgr.start("sess-001", "zone-1", rate.clone()).unwrap();

        let result = mgr.start("sess-002", "zone-1", rate);
        assert!(result.is_err());
        match result.unwrap_err() {
            SessionError::AlreadyActive => {}
            other => panic!("expected AlreadyActive, got: {other:?}"),
        }
        // Original session unchanged
        assert_eq!(mgr.get_status().unwrap().session_id, "sess-001");
    }

    /// TS-08-E6: stop() while no session returns NotActive.
    #[test]
    fn test_stop_session_while_no_session() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        let result = mgr.stop();
        assert!(result.is_err());
        match result.unwrap_err() {
            SessionError::NotActive => {}
            other => panic!("expected NotActive, got: {other:?}"),
        }
    }

    /// TS-08-E7: GetRate with no session returns None.
    #[test]
    fn test_get_rate_no_session() {
        let mgr = SessionManager::new(Some("zone-1".to_string()));
        assert!(mgr.get_rate().is_none());
    }

    // -----------------------------------------------------------------------
    // Low-level API tests (backward compatibility)
    // -----------------------------------------------------------------------

    /// TS-08-1, TS-08-P1: Lock event triggers idle -> starting transition.
    #[test]
    fn test_idle_to_starting_on_lock() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        assert_eq!(*mgr.state(), SessionState::Idle);
        mgr.try_start().expect("should transition to starting");
        assert_eq!(*mgr.state(), SessionState::Starting);
    }

    /// TS-08-1: Operator 200 OK transitions starting -> active.
    #[test]
    fn test_starting_to_active_on_operator_ok() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        mgr.try_start().unwrap();
        mgr.confirm_start("session-123".to_string());
        assert_eq!(*mgr.state(), SessionState::Active);
        assert_eq!(mgr.session_id(), Some("session-123"));
    }

    /// TS-08-2: Unlock event triggers active -> stopping transition.
    #[test]
    fn test_active_to_stopping_on_unlock() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        mgr.try_start().unwrap();
        mgr.confirm_start("session-123".to_string());
        mgr.try_stop().expect("should transition to stopping");
        assert_eq!(*mgr.state(), SessionState::Stopping);
    }

    /// TS-08-2: Operator 200 OK transitions stopping -> idle.
    #[test]
    fn test_stopping_to_idle_on_operator_ok() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        mgr.try_start().unwrap();
        mgr.confirm_start("session-123".to_string());
        mgr.try_stop().unwrap();
        mgr.confirm_stop();
        assert_eq!(*mgr.state(), SessionState::Idle);
        assert_eq!(mgr.session_id(), None);
    }

    /// TS-08-P1: Double lock is ignored (session already active).
    #[test]
    fn test_double_lock_ignored() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        mgr.try_start().unwrap();
        mgr.confirm_start("session-123".to_string());
        let result = mgr.try_start();
        assert!(result.is_err());
        assert_eq!(*mgr.state(), SessionState::Active);
    }

    /// TS-08-P2: Double unlock is ignored (no active session).
    #[test]
    fn test_double_unlock_ignored() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        mgr.try_start().unwrap();
        mgr.confirm_start("session-123".to_string());
        mgr.try_stop().unwrap();
        mgr.confirm_stop();
        let result = mgr.try_stop();
        assert!(result.is_err());
        assert_eq!(*mgr.state(), SessionState::Idle);
    }

    /// CP-6: Operator error during start transitions starting -> idle.
    #[test]
    fn test_starting_to_idle_on_operator_error() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        mgr.try_start().unwrap();
        assert_eq!(*mgr.state(), SessionState::Starting);
        mgr.fail_start();
        assert_eq!(*mgr.state(), SessionState::Idle);
        assert_eq!(mgr.session_id(), None);
    }

    /// Operator error during stop transitions stopping -> active
    /// (session state not updated on stop failure per 08-REQ-2.E2).
    #[test]
    fn test_stopping_to_active_on_operator_error() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        mgr.try_start().unwrap();
        mgr.confirm_start("session-123".to_string());
        mgr.try_stop().unwrap();
        mgr.fail_stop();
        assert_eq!(*mgr.state(), SessionState::Active);
    }

    /// is_active returns true only when state is Active.
    #[test]
    fn test_is_active() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        assert!(!mgr.is_active());

        mgr.try_start().unwrap();
        assert!(!mgr.is_active()); // Starting != Active

        mgr.confirm_start("sess-001".to_string());
        assert!(mgr.is_active());

        mgr.try_stop().unwrap();
        assert!(!mgr.is_active()); // Stopping != Active

        mgr.confirm_stop();
        assert!(!mgr.is_active());
    }
}
