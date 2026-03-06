//! Session state machine for the PARKING_OPERATOR_ADAPTOR.

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

    /// Returns the current zone ID, if any.
    pub fn zone_id(&self) -> Option<&str> {
        self.zone_id.as_deref()
    }

    /// Attempts to transition from Idle to Starting.
    /// Returns Err(AlreadyActive) if session is already active or starting.
    pub fn try_start(&mut self, zone_id: &str) -> Result<(), SessionError> {
        match self.state {
            SessionState::Idle => {
                self.state = SessionState::Starting;
                self.zone_id = Some(zone_id.to_string());
                Ok(())
            }
            SessionState::Active | SessionState::Starting => Err(SessionError::AlreadyActive),
            SessionState::Stopping => Err(SessionError::InvalidTransition),
        }
    }

    /// Confirms a successful start (Starting -> Active).
    pub fn confirm_start(&mut self, session_id: &str) {
        if self.state == SessionState::Starting {
            self.state = SessionState::Active;
            self.session_id = Some(session_id.to_string());
        }
    }

    /// Records a failed start (Starting -> Idle).
    pub fn fail_start(&mut self) {
        if self.state == SessionState::Starting {
            self.state = SessionState::Idle;
            self.session_id = None;
            self.zone_id = None;
        }
    }

    /// Attempts to transition from Active to Stopping.
    /// Returns the session_id on success, or Err(NoActiveSession) if no session is active.
    pub fn try_stop(&mut self) -> Result<String, SessionError> {
        match self.state {
            SessionState::Active => {
                let session_id = self
                    .session_id
                    .clone()
                    .ok_or(SessionError::InvalidTransition)?;
                self.state = SessionState::Stopping;
                Ok(session_id)
            }
            SessionState::Idle | SessionState::Stopping => Err(SessionError::NoActiveSession),
            SessionState::Starting => Err(SessionError::InvalidTransition),
        }
    }

    /// Confirms a successful stop (Stopping -> Idle).
    pub fn confirm_stop(&mut self) {
        if self.state == SessionState::Stopping {
            self.state = SessionState::Idle;
            self.session_id = None;
            self.zone_id = None;
        }
    }

    /// Records a failed stop (Stopping -> Idle to avoid stuck state).
    pub fn fail_stop(&mut self) {
        if self.state == SessionState::Stopping {
            self.state = SessionState::Idle;
            // session_id is kept as-is; the operator did not confirm stop
            self.session_id = None;
            self.zone_id = None;
        }
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
