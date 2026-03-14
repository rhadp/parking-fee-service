//! Session state management.
//!
//! Provides [`SessionManager`] — an in-memory state machine for the current
//! parking session.  All mutations are synchronous (`&mut self`) and the
//! caller is responsible for holding a `tokio::sync::Mutex` externally.
//!
//! Requirements: 08-REQ-1.2, 08-REQ-2.2, 08-REQ-3.E1, 08-REQ-3.E2,
//!               08-REQ-4.1, 08-REQ-4.2, 08-REQ-5.1, 08-REQ-5.2

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Pricing information for a parking session.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Rate {
    /// Pricing model: `"per_hour"` or `"flat_fee"`.
    pub rate_type: String,
    /// Price amount.
    pub amount: f64,
    /// ISO 4217 currency code (e.g. `"EUR"`).
    pub currency: String,
}

/// Session info returned by [`SessionManager::get_status`].
#[derive(Clone, Debug)]
pub struct SessionStatus {
    /// PARKING_OPERATOR-assigned session identifier.
    pub session_id: String,
    /// Parking zone identifier.
    pub zone_id: String,
    /// Unix timestamp when the session started.
    pub start_time: i64,
    /// Pricing details for this session.
    pub rate: Rate,
    /// Whether the session is currently active.
    pub active: bool,
}

/// Internal lifecycle state of the [`SessionManager`].
///
/// The state machine drives both the autonomous flow and the manual gRPC
/// override flow.  Autonomous transitions use `try_start` / `confirm_start` /
/// `fail_start` (and equivalents for stop) so that the HTTP call can be made
/// *outside* the mutex.  Manual transitions use the simpler `start` / `stop`
/// methods.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    /// No active session; ready to start.
    Idle,
    /// Autonomous start in progress (operator call outstanding).
    Starting,
    /// Session is active.
    Active,
    /// Autonomous stop in progress (operator call outstanding).
    Stopping,
}

/// Errors returned by [`SessionManager`] operations.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionError {
    /// Attempted to start a session that is already active.
    AlreadyActive,
    /// Attempted to stop when no session is active.
    NotActive,
    /// Unexpected state transition (internal logic error).
    InvalidTransition(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyActive => write!(f, "session already active"),
            Self::NotActive => write!(f, "no active session"),
            Self::InvalidTransition(msg) => write!(f, "invalid transition: {msg}"),
        }
    }
}

impl std::error::Error for SessionError {}

// ---------------------------------------------------------------------------
// SessionManager
// ---------------------------------------------------------------------------

/// In-memory session state machine.
///
/// # Stub
/// All mutating methods are **not yet implemented** and return errors or do
/// nothing.  Task group 3 replaces the stubs with the real implementation.
// zone_id, rate, and start_time are stored here for future use in get_status().
#[allow(dead_code)]
pub struct SessionManager {
    state: SessionState,
    session_id: Option<String>,
    zone_id: Option<String>,
    rate: Option<Rate>,
    start_time: Option<i64>,
}

impl SessionManager {
    /// Create a new `SessionManager` in the `Idle` state.
    ///
    /// `default_zone_id` is informational only in this stub.
    pub fn new(_default_zone_id: Option<String>) -> Self {
        Self {
            state: SessionState::Idle,
            session_id: None,
            zone_id: None,
            rate: None,
            start_time: None,
        }
    }

    /// Return the current lifecycle state.
    pub fn state(&self) -> &SessionState {
        &self.state
    }

    /// Return `true` if a session is currently active.
    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }

    /// Return the current session ID, if any.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    // -----------------------------------------------------------------------
    // Autonomous flow — state machine transitions
    // -----------------------------------------------------------------------

    /// Begin autonomous start: transition `Idle → Starting`.
    ///
    /// Returns `SessionError::AlreadyActive` if the session is not idle.
    ///
    /// # Stub — not yet implemented (task group 3)
    pub fn try_start(&mut self) -> Result<(), SessionError> {
        // STUB: task group 3 implements the real transition.
        if self.state != SessionState::Idle {
            return Err(SessionError::AlreadyActive);
        }
        // Stub keeps state as Idle (real impl would set Starting)
        Ok(())
    }

    /// Confirm autonomous start: store `session_id`, transition `Starting → Active`.
    ///
    /// # Stub — no-op (task group 3)
    pub fn confirm_start(&mut self, _session_id: String) {
        // STUB: no-op — real impl stores session_id and transitions to Active.
    }

    /// Autonomous start failed: transition `Starting → Idle`.
    ///
    /// # Stub — sets state to Idle (task group 3)
    pub fn fail_start(&mut self) {
        self.state = SessionState::Idle;
    }

    /// Begin autonomous stop: transition `Active → Stopping`.
    ///
    /// Returns `SessionError::NotActive` if there is no active session.
    ///
    /// # Stub — not yet implemented (task group 3)
    pub fn try_stop(&mut self) -> Result<(), SessionError> {
        // STUB: task group 3 implements the real transition.
        if self.state != SessionState::Active {
            return Err(SessionError::NotActive);
        }
        // Stub keeps state as Active (real impl would set Stopping)
        Ok(())
    }

    /// Confirm autonomous stop: clear session, transition `Stopping → Idle`.
    ///
    /// # Stub — no-op (task group 3)
    pub fn confirm_stop(&mut self) {
        // STUB: no-op — real impl clears session and transitions to Idle.
    }

    /// Autonomous stop failed: transition `Stopping → Active`.
    ///
    /// # Stub — resets to Active to allow retry (task group 3)
    pub fn fail_stop(&mut self) {
        self.state = SessionState::Active;
    }

    // -----------------------------------------------------------------------
    // Manual (gRPC) flow
    // -----------------------------------------------------------------------

    /// Atomically start a session (gRPC override / direct use).
    ///
    /// Returns `SessionError::AlreadyActive` if a session is already active.
    ///
    /// # Stub — not yet implemented (task group 3)
    pub fn start(
        &mut self,
        _session_id: &str,
        _zone_id: &str,
        _rate: Rate,
    ) -> Result<(), SessionError> {
        // STUB: returns AlreadyActive so tests fail (task group 3 implements real logic).
        Err(SessionError::AlreadyActive)
    }

    /// Atomically stop the current session (gRPC override / direct use).
    ///
    /// Returns `SessionError::NotActive` if no session is active.
    ///
    /// # Stub — not yet implemented (task group 3)
    pub fn stop(&mut self) -> Result<(), SessionError> {
        // STUB: returns NotActive so tests fail (task group 3 implements real logic).
        Err(SessionError::NotActive)
    }

    // -----------------------------------------------------------------------
    // Query methods
    // -----------------------------------------------------------------------

    /// Return the current session status, or `None` if idle.
    ///
    /// # Stub — always returns `None` (task group 3)
    pub fn get_status(&self) -> Option<SessionStatus> {
        // STUB: always returns None — real impl returns Some(SessionStatus{...}).
        None
    }

    /// Return the cached rate for the current session, or `None` if idle.
    ///
    /// # Stub — always returns `None` (task group 3)
    pub fn get_rate(&self) -> Option<Rate> {
        // STUB: always returns None — real impl returns Some(Rate{...}).
        None
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rate() -> Rate {
        Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.50,
            currency: "EUR".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // TS-08-2: Session state stored after start
    // -----------------------------------------------------------------------

    /// TS-08-2: After successful start, session_id, zone_id, start_time, and
    /// rate are stored in the session manager.
    #[test]
    fn test_session_state_stored_after_start() {
        let mut sm = SessionManager::new(None);
        let rate = make_rate();

        sm.start("sess-001", "zone-1", rate.clone()).unwrap();

        let status = sm.get_status().expect("get_status should return Some after start");
        assert_eq!(status.session_id, "sess-001");
        assert_eq!(status.zone_id, "zone-1");
        assert_eq!(status.rate.amount, 2.50);
        assert_eq!(status.rate.rate_type, "per_hour");
        assert!(status.active, "session should be active");
    }

    // -----------------------------------------------------------------------
    // TS-08-10: GetStatus active session
    // -----------------------------------------------------------------------

    /// TS-08-10: GetStatus returns session info when active.
    #[test]
    fn test_get_status_active_session() {
        let mut sm = SessionManager::new(None);
        let rate = make_rate();

        sm.start("sess-001", "zone-1", rate).unwrap();

        let status = sm.get_status().expect("get_status should return Some");
        assert_eq!(status.session_id, "sess-001");
        assert!(status.active);
    }

    // -----------------------------------------------------------------------
    // TS-08-11: GetStatus no session
    // -----------------------------------------------------------------------

    /// TS-08-11: GetStatus returns None when no session is active.
    #[test]
    fn test_get_status_no_session() {
        let sm = SessionManager::new(None);
        let status = sm.get_status();
        assert!(status.is_none(), "get_status should return None when idle");
    }

    // -----------------------------------------------------------------------
    // TS-08-12: GetRate active session
    // -----------------------------------------------------------------------

    /// TS-08-12: GetRate returns the cached rate when a session is active.
    #[test]
    fn test_get_rate_active_session() {
        let mut sm = SessionManager::new(None);
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.50,
            currency: "EUR".to_string(),
        };

        sm.start("sess-001", "zone-1", rate).unwrap();

        let r = sm.get_rate().expect("get_rate should return Some");
        assert_eq!(r.rate_type, "per_hour");
        assert_eq!(r.amount, 2.50);
        assert_eq!(r.currency, "EUR");
    }

    // -----------------------------------------------------------------------
    // TS-08-5: Session cleared after stop
    // -----------------------------------------------------------------------

    /// TS-08-5: After successful stop, session state is cleared.
    #[test]
    fn test_session_cleared_after_stop() {
        let mut sm = SessionManager::new(None);
        sm.start("sess-001", "zone-1", make_rate()).unwrap();
        sm.stop().unwrap();
        assert!(!sm.is_active(), "session should be inactive after stop");
        assert!(sm.get_status().is_none(), "get_status should be None after stop");
    }

    // -----------------------------------------------------------------------
    // TS-08-E5: StartSession while active → AlreadyActive
    // -----------------------------------------------------------------------

    /// TS-08-E5 (session layer): start() while active returns AlreadyActive.
    #[test]
    fn test_start_while_active() {
        let mut sm = SessionManager::new(None);
        sm.start("sess-001", "zone-1", make_rate()).unwrap();

        let err = sm
            .start("sess-002", "zone-1", make_rate())
            .unwrap_err();
        assert_eq!(err, SessionError::AlreadyActive);
    }

    // -----------------------------------------------------------------------
    // TS-08-E6: StopSession while no session → NotActive
    // -----------------------------------------------------------------------

    /// TS-08-E6 (session layer): stop() while idle returns NotActive.
    #[test]
    fn test_stop_while_idle() {
        let mut sm = SessionManager::new(None);
        let err = sm.stop().unwrap_err();
        assert_eq!(err, SessionError::NotActive);
    }

    // -----------------------------------------------------------------------
    // TS-08-E7: GetRate no session → None
    // -----------------------------------------------------------------------

    /// TS-08-E7: get_rate() returns None when no session is active.
    #[test]
    fn test_get_rate_no_session() {
        let sm = SessionManager::new(None);
        assert!(sm.get_rate().is_none());
    }
}
