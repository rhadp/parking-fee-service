/// Represents the current state of the parking session.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Idle,
    Starting,
    Active,
    Stopping,
}

/// Manages the parking session state machine.
/// All transitions are serialized through this manager.
pub struct SessionManager {
    state: SessionState,
    session_id: Option<String>,
    zone_id: Option<String>,
}

impl SessionManager {
    /// Create a new SessionManager in the Idle state.
    pub fn new(zone_id: Option<String>) -> Self {
        Self {
            state: SessionState::Idle,
            session_id: None,
            zone_id,
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
    /// Transitions from Starting to Active.
    pub fn confirm_start(&mut self, session_id: String) {
        self.state = SessionState::Active;
        self.session_id = Some(session_id);
    }

    /// Handle a failed session start from the operator.
    /// Transitions from Starting back to Idle.
    pub fn fail_start(&mut self) {
        self.state = SessionState::Idle;
        self.session_id = None;
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
    }

    /// Handle a failed session stop from the operator.
    /// Transitions from Stopping to Idle to avoid stuck state.
    pub fn fail_stop(&mut self) {
        self.state = SessionState::Idle;
    }
}

/// Errors that can occur during session state transitions.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("session already active")]
    AlreadyActive,
    #[error("no active session")]
    NoActiveSession,
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Second lock should return error since session is active
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
        // Second unlock should return error since no session active
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

    /// Operator error during stop transitions stopping -> idle.
    #[test]
    fn test_stopping_to_idle_on_operator_error() {
        let mut mgr = SessionManager::new(Some("zone-1".to_string()));
        mgr.try_start().unwrap();
        mgr.confirm_start("session-123".to_string());
        mgr.try_stop().unwrap();
        mgr.fail_stop();
        assert_eq!(*mgr.state(), SessionState::Idle);
    }
}
