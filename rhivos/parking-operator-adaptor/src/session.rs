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
// Helpers
// ---------------------------------------------------------------------------

fn current_unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ---------------------------------------------------------------------------
// SessionManager
// ---------------------------------------------------------------------------

/// In-memory session state machine.
///
/// Tracks the lifecycle of a single parking session.  Two flows are
/// supported:
///
/// * **Autonomous** — `try_start` → (HTTP call) → `confirm_start` or
///   `fail_start`; then `try_stop` → (HTTP call) → `confirm_stop` or
///   `fail_stop`.
/// * **Manual (gRPC)** — `start` and `stop` perform the full transition
///   atomically (no intermediate state).
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
    /// `default_zone_id` is stored for informational purposes.
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
    pub fn try_start(&mut self) -> Result<(), SessionError> {
        if self.state != SessionState::Idle {
            return Err(SessionError::AlreadyActive);
        }
        self.state = SessionState::Starting;
        Ok(())
    }

    /// Confirm autonomous start: store `session_id`, transition `Starting → Active`.
    pub fn confirm_start(&mut self, session_id: String) {
        self.session_id = Some(session_id);
        self.start_time = Some(current_unix_timestamp());
        self.state = SessionState::Active;
    }

    /// Autonomous start failed: transition `Starting → Idle`, clear partial state.
    pub fn fail_start(&mut self) {
        self.state = SessionState::Idle;
        self.session_id = None;
        self.zone_id = None;
        self.rate = None;
        self.start_time = None;
    }

    /// Begin autonomous stop: transition `Active → Stopping`.
    ///
    /// Returns `SessionError::NotActive` if there is no active session.
    pub fn try_stop(&mut self) -> Result<(), SessionError> {
        if self.state != SessionState::Active {
            return Err(SessionError::NotActive);
        }
        self.state = SessionState::Stopping;
        Ok(())
    }

    /// Confirm autonomous stop: clear session, transition `Stopping → Idle`.
    pub fn confirm_stop(&mut self) {
        self.state = SessionState::Idle;
        self.session_id = None;
        self.zone_id = None;
        self.rate = None;
        self.start_time = None;
    }

    /// Autonomous stop failed: transition `Stopping → Active`.
    pub fn fail_stop(&mut self) {
        self.state = SessionState::Active;
    }

    // -----------------------------------------------------------------------
    // Manual (gRPC) flow
    // -----------------------------------------------------------------------

    /// Atomically start a session (gRPC override / direct use).
    ///
    /// Stores `session_id`, `zone_id`, `rate`, and the current timestamp.
    /// Returns `SessionError::AlreadyActive` if a session is already active.
    pub fn start(
        &mut self,
        session_id: &str,
        zone_id: &str,
        rate: Rate,
    ) -> Result<(), SessionError> {
        if self.state != SessionState::Idle {
            return Err(SessionError::AlreadyActive);
        }
        self.session_id = Some(session_id.to_string());
        self.zone_id = Some(zone_id.to_string());
        self.rate = Some(rate);
        self.start_time = Some(current_unix_timestamp());
        self.state = SessionState::Active;
        Ok(())
    }

    /// Atomically stop the current session (gRPC override / direct use).
    ///
    /// Clears all stored session data.
    /// Returns `SessionError::NotActive` if no session is active.
    pub fn stop(&mut self) -> Result<(), SessionError> {
        if self.state != SessionState::Active {
            return Err(SessionError::NotActive);
        }
        self.state = SessionState::Idle;
        self.session_id = None;
        self.zone_id = None;
        self.rate = None;
        self.start_time = None;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Query methods
    // -----------------------------------------------------------------------

    /// Return the current session status, or `None` if no session is active.
    pub fn get_status(&self) -> Option<SessionStatus> {
        if self.state != SessionState::Active {
            return None;
        }
        Some(SessionStatus {
            session_id: self.session_id.clone()?,
            zone_id: self.zone_id.clone().unwrap_or_default(),
            start_time: self.start_time.unwrap_or(0),
            rate: self.rate.clone()?,
            active: true,
        })
    }

    /// Return the cached rate for the current session, or `None` if no
    /// session is active.
    pub fn get_rate(&self) -> Option<Rate> {
        if self.state != SessionState::Active {
            return None;
        }
        self.rate.clone()
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

    // -----------------------------------------------------------------------
    // Autonomous flow transitions
    // -----------------------------------------------------------------------

    /// Verify try_start transitions Idle → Starting.
    #[test]
    fn test_try_start_transitions_to_starting() {
        let mut sm = SessionManager::new(None);
        assert_eq!(*sm.state(), SessionState::Idle);
        sm.try_start().unwrap();
        assert_eq!(*sm.state(), SessionState::Starting);
    }

    /// Verify try_start fails when not Idle.
    #[test]
    fn test_try_start_fails_when_not_idle() {
        let mut sm = SessionManager::new(None);
        sm.start("s-1", "z-1", make_rate()).unwrap(); // Active
        let err = sm.try_start().unwrap_err();
        assert_eq!(err, SessionError::AlreadyActive);
    }

    /// Verify confirm_start stores session_id and transitions to Active.
    #[test]
    fn test_confirm_start_transitions_to_active() {
        let mut sm = SessionManager::new(None);
        sm.try_start().unwrap();
        sm.confirm_start("sess-auto-001".to_string());
        assert_eq!(*sm.state(), SessionState::Active);
        assert_eq!(sm.session_id(), Some("sess-auto-001"));
        assert!(sm.is_active());
    }

    /// Verify fail_start resets to Idle.
    #[test]
    fn test_fail_start_resets_to_idle() {
        let mut sm = SessionManager::new(None);
        sm.try_start().unwrap();
        sm.fail_start();
        assert_eq!(*sm.state(), SessionState::Idle);
        assert!(!sm.is_active());
    }

    /// Verify try_stop transitions Active → Stopping.
    #[test]
    fn test_try_stop_transitions_to_stopping() {
        let mut sm = SessionManager::new(None);
        sm.start("s-1", "z-1", make_rate()).unwrap();
        sm.try_stop().unwrap();
        assert_eq!(*sm.state(), SessionState::Stopping);
    }

    /// Verify try_stop fails when not Active.
    #[test]
    fn test_try_stop_fails_when_not_active() {
        let mut sm = SessionManager::new(None);
        let err = sm.try_stop().unwrap_err();
        assert_eq!(err, SessionError::NotActive);
    }

    /// Verify confirm_stop clears session and transitions to Idle.
    #[test]
    fn test_confirm_stop_clears_and_transitions_to_idle() {
        let mut sm = SessionManager::new(None);
        sm.start("s-1", "z-1", make_rate()).unwrap();
        sm.try_stop().unwrap();
        sm.confirm_stop();
        assert_eq!(*sm.state(), SessionState::Idle);
        assert!(!sm.is_active());
        assert!(sm.session_id().is_none());
        assert!(sm.get_status().is_none());
    }

    /// Verify fail_stop transitions Stopping → Active.
    #[test]
    fn test_fail_stop_returns_to_active() {
        let mut sm = SessionManager::new(None);
        sm.start("s-1", "z-1", make_rate()).unwrap();
        sm.try_stop().unwrap();
        sm.fail_stop();
        assert_eq!(*sm.state(), SessionState::Active);
        assert!(sm.is_active());
    }
}
