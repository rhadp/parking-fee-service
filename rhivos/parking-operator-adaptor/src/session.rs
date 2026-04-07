/// Rate information from the PARKING_OPERATOR.
#[derive(Clone, Debug, PartialEq)]
pub struct Rate {
    pub rate_type: String, // "per_hour" | "flat_fee"
    pub amount: f64,
    pub currency: String,
}

/// In-memory parking session state.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionState {
    pub session_id: String,
    pub zone_id: String,
    pub start_time: i64, // Unix timestamp
    pub rate: Rate,
    pub active: bool,
}

/// Manages the in-memory parking session.
///
/// Wraps an `Option<SessionState>`. When no session is active,
/// the inner value is `None`.
pub struct Session {
    state: Option<SessionState>,
}

impl Session {
    /// Create a new Session with no active session.
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Returns `true` if a session is currently active.
    pub fn is_active(&self) -> bool {
        self.state.as_ref().is_some_and(|s| s.active)
    }

    /// Start a new session with the given fields.
    pub fn start(
        &mut self,
        session_id: String,
        zone_id: String,
        start_time: i64,
        rate: Rate,
    ) {
        self.state = Some(SessionState {
            session_id,
            zone_id,
            start_time,
            rate,
            active: true,
        });
    }

    /// Stop the current session. Returns the session_id if was active.
    pub fn stop(&mut self) -> Option<String> {
        let session_id = self.state.as_ref().map(|s| s.session_id.clone());
        self.state = None;
        session_id
    }

    /// Get the current session state, if active.
    pub fn status(&self) -> Option<&SessionState> {
        self.state.as_ref().filter(|s| s.active)
    }

    /// Get the current rate, if a session is active.
    pub fn rate(&self) -> Option<&Rate> {
        self.state.as_ref().filter(|s| s.active).map(|s| &s.rate)
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rate() -> Rate {
        Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        }
    }

    fn flat_rate() -> Rate {
        Rate {
            rate_type: "flat_fee".to_string(),
            amount: 5.0,
            currency: "EUR".to_string(),
        }
    }

    // TS-08-22: Session State Fields
    #[test]
    fn test_session_start_stop_fields() {
        let mut session = Session::new();
        assert!(!session.is_active());

        session.start(
            "s1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            sample_rate(),
        );

        let state = session.status().expect("should have active session");
        assert_eq!(state.session_id, "s1");
        assert_eq!(state.zone_id, "zone-a");
        assert_eq!(state.start_time, 1_700_000_000);
        assert_eq!(state.rate.rate_type, "per_hour");
        assert_eq!(state.rate.amount, 2.5);
        assert_eq!(state.rate.currency, "EUR");
        assert!(state.active);
        assert!(session.is_active());

        let stopped_id = session.stop();
        assert_eq!(stopped_id, Some("s1".to_string()));
        assert!(!session.is_active());
        assert!(session.status().is_none());
    }

    // TS-08-4: GetStatus Returns Active Session
    #[test]
    fn test_get_status_active() {
        let mut session = Session::new();
        session.start(
            "sess-1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            sample_rate(),
        );

        let state = session.status().expect("should have active session");
        assert!(state.active);
        assert_eq!(state.session_id, "sess-1");
        assert_eq!(state.zone_id, "zone-a");
        assert_eq!(state.start_time, 1_700_000_000);
        assert_eq!(state.rate.rate_type, "per_hour");
    }

    // TS-08-5: GetStatus Returns Inactive When No Session
    #[test]
    fn test_get_status_inactive() {
        let session = Session::new();
        assert!(!session.is_active());
        assert!(session.status().is_none());
    }

    // TS-08-6: GetRate Returns Cached Rate
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
        assert_eq!(rate.amount, 5.0);
        assert_eq!(rate.currency, "EUR");
    }

    // TS-08-7: GetRate Returns Empty When No Session
    #[test]
    fn test_get_rate_inactive() {
        let session = Session::new();
        assert!(session.rate().is_none());
    }
}
