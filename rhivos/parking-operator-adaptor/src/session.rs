/// Rate information from PARKING_OPERATOR.
#[derive(Debug, Clone, PartialEq)]
pub struct Rate {
    /// Rate type: "per_hour" or "flat_fee".
    pub rate_type: String,
    /// Rate amount.
    pub amount: f64,
    /// Currency code (e.g. "EUR").
    pub currency: String,
}

/// In-memory parking session state.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionState {
    pub session_id: String,
    pub zone_id: String,
    pub start_time: i64,
    pub rate: Rate,
    pub active: bool,
}

/// In-memory session manager.
///
/// Wraps an `Option<SessionState>` and provides start/stop/query
/// operations for the active parking session.
#[allow(dead_code)]
pub struct Session {
    state: Option<SessionState>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    /// Create a new session manager with no active session.
    pub fn new() -> Self {
        todo!("Session::new not yet implemented")
    }

    /// Returns `true` if a parking session is currently active.
    pub fn is_active(&self) -> bool {
        todo!("Session::is_active not yet implemented")
    }

    /// Start a parking session with the given parameters.
    ///
    /// Populates all session fields and sets `active` to `true`.
    pub fn start(
        &mut self,
        _session_id: String,
        _zone_id: String,
        _start_time: i64,
        _rate: Rate,
    ) {
        todo!("Session::start not yet implemented")
    }

    /// Stop the active parking session.
    ///
    /// Returns the `session_id` if a session was active, or `None`
    /// if no session was active. Clears the session state.
    pub fn stop(&mut self) -> Option<String> {
        todo!("Session::stop not yet implemented")
    }

    /// Returns the current session state, or `None` if no session.
    pub fn status(&self) -> Option<&SessionState> {
        todo!("Session::status not yet implemented")
    }

    /// Returns the rate from the active session, or `None` if no session.
    pub fn rate(&self) -> Option<&Rate> {
        todo!("Session::rate not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rate() -> Rate {
        Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.50,
            currency: "EUR".to_string(),
        }
    }

    fn flat_rate() -> Rate {
        Rate {
            rate_type: "flat_fee".to_string(),
            amount: 5.00,
            currency: "EUR".to_string(),
        }
    }

    // TS-08-22: Session State Fields
    // Verify session state is correctly populated and cleared.
    #[test]
    fn test_session_start_stop_fields() {
        let mut session = Session::new();
        assert!(!session.is_active(), "new session should be inactive");

        session.start(
            "s1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            sample_rate(),
        );

        let state = session.status().expect("session should have state after start");
        assert_eq!(state.session_id, "s1");
        assert_eq!(state.zone_id, "zone-a");
        assert_eq!(state.start_time, 1_700_000_000);
        assert_eq!(state.rate.rate_type, "per_hour");
        assert!(state.active, "session should be active after start");

        let stopped_id = session.stop();
        assert_eq!(stopped_id, Some("s1".to_string()));
        assert!(!session.is_active(), "session should be inactive after stop");
        assert!(session.status().is_none(), "session state should be cleared after stop");
    }

    // TS-08-4: GetStatus Returns Active Session
    // Verify status returns session details when active.
    #[test]
    fn test_get_status_active() {
        let mut session = Session::new();
        session.start(
            "sess-1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            sample_rate(),
        );

        let state = session.status().expect("should have status");
        assert!(session.is_active());
        assert_eq!(state.session_id, "sess-1");
        assert_eq!(state.zone_id, "zone-a");
        assert_eq!(state.start_time, 1_700_000_000);
        assert_eq!(state.rate.rate_type, "per_hour");
    }

    // TS-08-5: GetStatus Returns Inactive When No Session
    // Verify status returns None when no session exists.
    #[test]
    fn test_get_status_inactive() {
        let session = Session::new();
        assert!(!session.is_active());
        assert!(session.status().is_none());
    }

    // TS-08-6: GetRate Returns Cached Rate
    // Verify rate returns the rate from the active session.
    #[test]
    fn test_get_rate_active() {
        let mut session = Session::new();
        session.start(
            "sess-1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            flat_rate(),
        );

        let rate = session.rate().expect("should have rate");
        assert_eq!(rate.rate_type, "flat_fee");
        assert!((rate.amount - 5.00).abs() < f64::EPSILON);
        assert_eq!(rate.currency, "EUR");
    }

    // TS-08-7: GetRate Returns Empty When No Session
    // Verify rate returns None when no session is active.
    #[test]
    fn test_get_rate_inactive() {
        let session = Session::new();
        assert!(session.rate().is_none());
    }

    // TS-08-22 additional: Verify stop on inactive session returns None.
    #[test]
    fn test_stop_no_active_session() {
        let mut session = Session::new();
        let result = session.stop();
        assert_eq!(result, None);
    }
}
