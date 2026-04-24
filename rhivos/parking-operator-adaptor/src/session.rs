/// Rate information from the PARKING_OPERATOR.
#[derive(Debug, Clone, PartialEq)]
pub struct Rate {
    /// Rate type: "per_hour" or "flat_fee".
    pub rate_type: String,
    /// Monetary amount (e.g. 2.50).
    pub amount: f64,
    /// Currency code (e.g. "EUR").
    pub currency: String,
}

/// In-memory parking session state.
#[derive(Debug, Clone)]
pub struct SessionState {
    pub session_id: String,
    pub zone_id: String,
    pub start_time: i64,
    pub rate: Rate,
    pub active: bool,
}

/// Manages in-memory parking session state.
///
/// Wraps an `Option<SessionState>` and provides typed operations
/// for starting, stopping, and querying sessions.
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
        todo!()
    }

    /// Returns `true` if a parking session is currently active.
    pub fn is_active(&self) -> bool {
        todo!()
    }

    /// Start a new parking session, populating all fields.
    pub fn start(&mut self, _session_id: String, _zone_id: String, _start_time: i64, _rate: Rate) {
        todo!()
    }

    /// Stop the active parking session.
    ///
    /// Returns the session_id if a session was active, or `None` if no
    /// session was active.
    pub fn stop(&mut self) -> Option<String> {
        todo!()
    }

    /// Returns a reference to the current session state, or `None` if no
    /// session is active.
    pub fn status(&self) -> Option<&SessionState> {
        todo!()
    }

    /// Returns a reference to the rate from the active session, or `None`
    /// if no session is active.
    pub fn rate(&self) -> Option<&Rate> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TS-08-22: Session State Fields
    // Validates: [08-REQ-6.1], [08-REQ-6.2], [08-REQ-6.3]
    #[test]
    fn test_session_start_stop_fields() {
        // GIVEN a new session manager
        let mut session = Session::new();

        // THEN no session is active initially
        assert!(!session.is_active());
        assert!(session.status().is_none());

        // WHEN a session is started
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        };
        session.start("s1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        // THEN all fields are populated and active is true
        assert!(session.is_active());
        let state = session.status().expect("session should be active");
        assert_eq!(state.session_id, "s1");
        assert_eq!(state.zone_id, "zone-a");
        assert_eq!(state.start_time, 1_700_000_000);
        assert_eq!(state.rate.rate_type, "per_hour");
        assert!((state.rate.amount - 2.5).abs() < f64::EPSILON);
        assert_eq!(state.rate.currency, "EUR");
        assert!(state.active);

        // WHEN the session is stopped
        let stopped_id = session.stop();

        // THEN session is inactive and fields are cleared
        assert_eq!(stopped_id, Some("s1".to_string()));
        assert!(!session.is_active());
        assert!(session.status().is_none());
    }

    // TS-08-4: GetStatus Returns Active Session
    // Validates: [08-REQ-1.4]
    #[test]
    fn test_get_status_active() {
        // GIVEN an active session
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

        // WHEN status is queried
        let state = session.status().expect("session should be active");

        // THEN response contains all session details
        assert!(session.is_active());
        assert_eq!(state.session_id, "sess-1");
        assert_eq!(state.zone_id, "zone-a");
        assert_eq!(state.start_time, 1_700_000_000);
        assert_eq!(state.rate.rate_type, "per_hour");
    }

    // TS-08-5: GetStatus Returns Inactive When No Session
    // Validates: [08-REQ-1.4]
    #[test]
    fn test_get_status_inactive() {
        // GIVEN no active session
        let session = Session::new();

        // WHEN status is queried
        let status = session.status();

        // THEN active is false and status is None
        assert!(!session.is_active());
        assert!(status.is_none());
    }

    // TS-08-6: GetRate Returns Cached Rate
    // Validates: [08-REQ-1.5]
    #[test]
    fn test_get_rate_active() {
        // GIVEN an active session with flat_fee rate
        let mut session = Session::new();
        let rate = Rate {
            rate_type: "flat_fee".to_string(),
            amount: 5.0,
            currency: "EUR".to_string(),
        };
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        // WHEN rate is queried
        let r = session.rate().expect("rate should be present");

        // THEN rate matches the session rate
        assert_eq!(r.rate_type, "flat_fee");
        assert!((r.amount - 5.0).abs() < f64::EPSILON);
        assert_eq!(r.currency, "EUR");
    }

    // TS-08-7: GetRate Returns Empty When No Session
    // Validates: [08-REQ-1.5]
    #[test]
    fn test_get_rate_inactive() {
        // GIVEN no active session
        let session = Session::new();

        // WHEN rate is queried
        let rate = session.rate();

        // THEN rate is None
        assert!(rate.is_none());
    }
}
