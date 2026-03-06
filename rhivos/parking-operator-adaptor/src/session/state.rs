//! Session state machine for the PARKING_OPERATOR_ADAPTOR.
//! Stub: logic will be implemented in task group 2.

/// Possible states for a parking session.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Idle,
    Starting,
    Active,
    Stopping,
}

/// Error type for session state transitions.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionError {
    AlreadyActive,
    NoActiveSession,
    InvalidTransition,
}

/// Manages parking session state transitions.
pub struct SessionManager {
    state: SessionState,
    session_id: Option<String>,
    #[allow(dead_code)]
    zone_id: Option<String>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    /// Creates a new SessionManager in the Idle state.
    pub fn new() -> Self {
        Self {
            state: SessionState::Idle,
            session_id: None,
            zone_id: None,
        }
    }

    /// Returns the current session state.
    pub fn state(&self) -> &SessionState {
        &self.state
    }

    /// Returns the current session ID, if any.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Attempts to transition from Idle to Starting.
    /// Returns Err(AlreadyActive) if session is already active.
    pub fn try_start(&mut self, _zone_id: &str) -> Result<(), SessionError> {
        // Stub: not yet implemented
        Err(SessionError::InvalidTransition)
    }

    /// Confirms a successful start (Starting -> Active).
    pub fn confirm_start(&mut self, _session_id: &str) {
        // Stub: not yet implemented
    }

    /// Records a failed start (Starting -> Idle).
    pub fn fail_start(&mut self) {
        // Stub: not yet implemented
    }

    /// Attempts to transition from Active to Stopping.
    /// Returns Err(NoActiveSession) if no session is active.
    pub fn try_stop(&mut self) -> Result<String, SessionError> {
        // Stub: not yet implemented
        Err(SessionError::InvalidTransition)
    }

    /// Confirms a successful stop (Stopping -> Idle).
    pub fn confirm_stop(&mut self) {
        // Stub: not yet implemented
    }

    /// Records a failed stop (Stopping -> Idle to avoid stuck state).
    pub fn fail_stop(&mut self) {
        // Stub: not yet implemented
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-08-1 / TS-08-P1: Idle -> Starting on lock event / StartSession.
    #[test]
    fn test_idle_to_starting_on_lock() {
        let mut mgr = SessionManager::new();
        assert_eq!(*mgr.state(), SessionState::Idle);
        let result = mgr.try_start("zone-demo-1");
        assert!(result.is_ok(), "try_start should succeed from Idle state");
        assert_eq!(*mgr.state(), SessionState::Starting);
    }

    /// TS-08-1: Starting -> Active on operator 200 OK.
    #[test]
    fn test_starting_to_active_on_operator_ok() {
        let mut mgr = SessionManager::new();
        mgr.try_start("zone-demo-1").unwrap();
        mgr.confirm_start("sess-123");
        assert_eq!(*mgr.state(), SessionState::Active);
        assert_eq!(mgr.session_id(), Some("sess-123"));
    }

    /// TS-08-2: Active -> Stopping on unlock event / StopSession.
    #[test]
    fn test_active_to_stopping_on_unlock() {
        let mut mgr = SessionManager::new();
        mgr.try_start("zone-demo-1").unwrap();
        mgr.confirm_start("sess-123");
        let session_id = mgr.try_stop();
        assert!(session_id.is_ok(), "try_stop should succeed from Active state");
        assert_eq!(session_id.unwrap(), "sess-123");
        assert_eq!(*mgr.state(), SessionState::Stopping);
    }

    /// TS-08-2: Stopping -> Idle on operator 200 OK.
    #[test]
    fn test_stopping_to_idle_on_operator_ok() {
        let mut mgr = SessionManager::new();
        mgr.try_start("zone-demo-1").unwrap();
        mgr.confirm_start("sess-123");
        mgr.try_stop().unwrap();
        mgr.confirm_stop();
        assert_eq!(*mgr.state(), SessionState::Idle);
        assert!(mgr.session_id().is_none());
    }

    /// TS-08-P1: Double lock is ignored (session already active).
    #[test]
    fn test_double_lock_ignored() {
        let mut mgr = SessionManager::new();
        mgr.try_start("zone-demo-1").unwrap();
        mgr.confirm_start("sess-123");
        assert_eq!(*mgr.state(), SessionState::Active);
        // Second lock attempt should return AlreadyActive
        let result = mgr.try_start("zone-demo-1");
        assert_eq!(result, Err(SessionError::AlreadyActive));
        assert_eq!(*mgr.state(), SessionState::Active);
    }

    /// TS-08-P2: Double unlock is ignored (no active session).
    #[test]
    fn test_double_unlock_ignored() {
        let mut mgr = SessionManager::new();
        // Attempt to stop when idle should fail
        let result = mgr.try_stop();
        assert_eq!(result, Err(SessionError::NoActiveSession));
        assert_eq!(*mgr.state(), SessionState::Idle);
    }

    /// CP-6: Starting -> Idle on operator error.
    #[test]
    fn test_starting_to_idle_on_operator_error() {
        let mut mgr = SessionManager::new();
        mgr.try_start("zone-demo-1").unwrap();
        assert_eq!(*mgr.state(), SessionState::Starting);
        mgr.fail_start();
        assert_eq!(*mgr.state(), SessionState::Idle);
        assert!(mgr.session_id().is_none());
    }
}
