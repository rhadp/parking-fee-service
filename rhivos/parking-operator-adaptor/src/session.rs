//! In-memory session state management.

/// Rate information returned by the PARKING_OPERATOR.
#[derive(Debug, Clone, PartialEq)]
pub struct Rate {
    /// "per_hour" or "flat_fee"
    pub rate_type: String,
    pub amount: f64,
    pub currency: String,
}

/// Full in-memory parking session record.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionState {
    pub session_id: String,
    pub zone_id: String,
    /// Unix timestamp (seconds).
    pub start_time: i64,
    pub rate: Rate,
    pub active: bool,
}

/// Session manager wrapping an optional active session.
#[allow(dead_code)]
pub struct Session {
    state: Option<SessionState>,
}

impl Session {
    /// Create a new session manager with no active session.
    pub fn new() -> Self {
        todo!("Session::new not yet implemented")
    }

    /// Return `true` if a session is currently active.
    pub fn is_active(&self) -> bool {
        todo!("Session::is_active not yet implemented")
    }

    /// Record a successfully started session.
    pub fn start(
        &mut self,
        _session_id: String,
        _zone_id: String,
        _start_time: i64,
        _rate: Rate,
    ) {
        todo!("Session::start not yet implemented")
    }

    /// Clear the active session record (called after a successful stop).
    pub fn stop(&mut self) {
        todo!("Session::stop not yet implemented")
    }

    /// Return a reference to the current session state, or `None`.
    pub fn status(&self) -> Option<&SessionState> {
        todo!("Session::status not yet implemented")
    }

    /// Return a reference to the cached rate, or `None` when no session is active.
    pub fn rate(&self) -> Option<&Rate> {
        todo!("Session::rate not yet implemented")
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

    fn make_rate(rate_type: &str, amount: f64, currency: &str) -> Rate {
        Rate {
            rate_type: rate_type.to_owned(),
            amount,
            currency: currency.to_owned(),
        }
    }

    // TS-08-22: Session State Fields
    #[test]
    fn test_session_start_stop_fields() {
        let mut session = Session::new();
        assert!(!session.is_active());
        assert!(session.status().is_none());

        let rate = make_rate("per_hour", 2.5, "EUR");
        session.start(
            "s1".to_owned(),
            "zone-a".to_owned(),
            1_700_000_000,
            rate.clone(),
        );

        let state = session.status().expect("session should be active after start");
        assert_eq!(state.session_id, "s1");
        assert_eq!(state.zone_id, "zone-a");
        assert_eq!(state.start_time, 1_700_000_000);
        assert_eq!(state.rate.rate_type, "per_hour");
        assert!(state.active);
        assert!(session.is_active());

        session.stop();
        assert!(!session.is_active());
        assert!(session.status().is_none());
    }

    // TS-08-4: GetStatus Returns Active Session
    #[test]
    fn test_get_status_active() {
        let mut session = Session::new();
        let rate = make_rate("per_hour", 2.5, "EUR");
        session.start(
            "sess-1".to_owned(),
            "zone-a".to_owned(),
            1_700_000_000,
            rate,
        );

        let status = session.status().expect("status should be Some when active");
        assert!(status.active);
        assert_eq!(status.session_id, "sess-1");
        assert_eq!(status.zone_id, "zone-a");
        assert_eq!(status.start_time, 1_700_000_000);
        assert_eq!(status.rate.rate_type, "per_hour");
    }

    // TS-08-5: GetStatus Returns Inactive When No Session
    #[test]
    fn test_get_status_inactive() {
        let session = Session::new();
        assert!(session.status().is_none());
        assert!(!session.is_active());
    }

    // TS-08-6: GetRate Returns Cached Rate
    #[test]
    fn test_get_rate_active() {
        let mut session = Session::new();
        let rate = make_rate("flat_fee", 5.0, "EUR");
        session.start("sess-1".to_owned(), "zone-a".to_owned(), 1_000, rate);

        let r = session.rate().expect("rate should be Some when active");
        assert_eq!(r.rate_type, "flat_fee");
        assert_eq!(r.amount, 5.0);
        assert_eq!(r.currency, "EUR");
    }

    // TS-08-7: GetRate Returns Empty When No Session
    #[test]
    fn test_get_rate_inactive() {
        let session = Session::new();
        assert!(session.rate().is_none());
    }
}
