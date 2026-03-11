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
    _state: SessionState,
    _session_id: Option<String>,
    _zone_id: Option<String>,
}

impl SessionManager {
    /// Create a new SessionManager in the Idle state.
    pub fn new(zone_id: Option<String>) -> Self {
        Self {
            _state: SessionState::Idle,
            _session_id: None,
            _zone_id: zone_id,
        }
    }

    /// Returns the current session state.
    pub fn state(&self) -> &SessionState {
        // Stub: will be implemented in task group 2
        todo!("SessionManager::state not yet implemented")
    }

    /// Returns the current session ID, if any.
    pub fn session_id(&self) -> Option<&str> {
        // Stub: will be implemented in task group 2
        todo!("SessionManager::session_id not yet implemented")
    }

    /// Attempt to transition from Idle to Starting.
    /// Returns Ok(()) if transition is valid, Err if session already active.
    pub fn try_start(&mut self) -> Result<(), SessionError> {
        // Stub: will be implemented in task group 2
        todo!("SessionManager::try_start not yet implemented")
    }

    /// Confirm a successful session start from the operator.
    /// Transitions from Starting to Active.
    pub fn confirm_start(&mut self, _session_id: String) {
        // Stub: will be implemented in task group 2
        todo!("SessionManager::confirm_start not yet implemented")
    }

    /// Handle a failed session start from the operator.
    /// Transitions from Starting back to Idle.
    pub fn fail_start(&mut self) {
        // Stub: will be implemented in task group 2
        todo!("SessionManager::fail_start not yet implemented")
    }

    /// Attempt to transition from Active to Stopping.
    /// Returns Ok(()) if transition is valid, Err if no session active.
    pub fn try_stop(&mut self) -> Result<(), SessionError> {
        // Stub: will be implemented in task group 2
        todo!("SessionManager::try_stop not yet implemented")
    }

    /// Confirm a successful session stop from the operator.
    /// Transitions from Stopping to Idle.
    pub fn confirm_stop(&mut self) {
        // Stub: will be implemented in task group 2
        todo!("SessionManager::confirm_stop not yet implemented")
    }

    /// Handle a failed session stop from the operator.
    /// Transitions from Stopping to Idle to avoid stuck state.
    pub fn fail_stop(&mut self) {
        // Stub: will be implemented in task group 2
        todo!("SessionManager::fail_stop not yet implemented")
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
