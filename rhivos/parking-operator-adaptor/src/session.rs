/// Rate information from the PARKING_OPERATOR.
#[derive(Debug, Clone, PartialEq)]
pub struct Rate {
    /// Rate type: "per_hour" or "flat_fee".
    pub rate_type: String,
    /// Rate amount (e.g. 2.50).
    pub amount: f64,
    /// Currency code (e.g. "EUR").
    pub currency: String,
}

/// In-memory parking session state.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionState {
    /// Session identifier from the PARKING_OPERATOR.
    pub session_id: String,
    /// Parking zone identifier.
    pub zone_id: String,
    /// Session start time as Unix timestamp.
    pub start_time: i64,
    /// Rate information.
    pub rate: Rate,
    /// Whether the session is currently active.
    pub active: bool,
}

/// In-memory session manager.
///
/// Wraps an `Option<SessionState>` and provides methods for
/// starting, stopping, and querying the session.
pub struct Session {
    state: Option<SessionState>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    /// Create a new Session with no active session.
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Returns true if a session is currently active.
    pub fn is_active(&self) -> bool {
        self.state.as_ref().is_some_and(|s| s.active)
    }

    /// Start a new session with the given parameters.
    ///
    /// Populates all fields and sets active to true.
    pub fn start(&mut self, session_id: String, zone_id: String, start_time: i64, rate: Rate) {
        self.state = Some(SessionState {
            session_id,
            zone_id,
            start_time,
            rate,
            active: true,
        });
    }

    /// Stop the active session.
    ///
    /// Returns the session_id if a session was active, None otherwise.
    /// Clears all session state.
    pub fn stop(&mut self) -> Option<String> {
        let session_id = self
            .state
            .as_ref()
            .filter(|s| s.active)
            .map(|s| s.session_id.clone());
        self.state = None;
        session_id
    }

    /// Get the current session state, if any.
    pub fn status(&self) -> Option<&SessionState> {
        self.state.as_ref().filter(|s| s.active)
    }

    /// Get the current rate, if any.
    pub fn rate(&self) -> Option<&Rate> {
        self.status().map(|s| &s.rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rate() -> Rate {
        Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        }
    }

    // TS-08-22: Verify session state is correctly populated and cleared.
    #[test]
    fn test_session_start_stop_fields() {
        let mut session = Session::new();
        assert!(!session.is_active(), "new session should be inactive");

        let rate = make_rate();
        session.start("s1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        let state = session.status().expect("session should have state after start");
        assert_eq!(state.session_id, "s1");
        assert_eq!(state.zone_id, "zone-a");
        assert_eq!(state.start_time, 1_700_000_000);
        assert_eq!(state.rate.rate_type, "per_hour");
        assert_eq!(state.rate.amount, 2.5);
        assert_eq!(state.rate.currency, "EUR");
        assert!(state.active, "session state should be active");
        assert!(session.is_active(), "session should report active");

        let stopped_id = session.stop();
        assert_eq!(stopped_id, Some("s1".to_string()));
        assert!(!session.is_active(), "session should be inactive after stop");
        assert!(session.status().is_none(), "status should be None after stop");
    }

    // TS-08-4: Verify GetStatus returns session details when active.
    #[test]
    fn test_get_status_active() {
        let mut session = Session::new();
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        };
        session.start(
            "sess-1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            rate,
        );

        let state = session.status().expect("should have status");
        assert!(session.is_active());
        assert_eq!(state.session_id, "sess-1");
        assert_eq!(state.zone_id, "zone-a");
        assert_eq!(state.start_time, 1_700_000_000);
        assert_eq!(state.rate.rate_type, "per_hour");
    }

    // TS-08-5: Verify GetStatus returns inactive when no session.
    #[test]
    fn test_get_status_inactive() {
        let session = Session::new();
        assert!(!session.is_active());
        assert!(session.status().is_none());
    }

    // TS-08-6: Verify GetRate returns the rate from active session.
    #[test]
    fn test_get_rate_active() {
        let mut session = Session::new();
        let rate = Rate {
            rate_type: "flat_fee".to_string(),
            amount: 5.0,
            currency: "EUR".to_string(),
        };
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        let r = session.rate().expect("should have rate");
        assert_eq!(r.rate_type, "flat_fee");
        assert_eq!(r.amount, 5.0);
        assert_eq!(r.currency, "EUR");
    }

    // TS-08-7: Verify GetRate returns None when no session.
    #[test]
    fn test_get_rate_inactive() {
        let session = Session::new();
        assert!(session.rate().is_none(), "rate should be None when no session");
    }
}
