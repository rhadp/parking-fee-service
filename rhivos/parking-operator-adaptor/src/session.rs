//! In-memory parking session state.
//!
//! Maintains the active parking session record per 08-REQ-6.1 through 08-REQ-6.3.

#![allow(dead_code)]

/// Parking rate information returned by the PARKING_OPERATOR.
#[derive(Debug, Clone)]
pub struct Rate {
    /// "per_hour" or "flat_fee".
    pub rate_type: String,
    pub amount: f64,
    pub currency: String,
}

/// Full in-memory parking session record (08-REQ-6.1).
#[derive(Debug, Clone)]
pub struct SessionState {
    pub session_id: String,
    pub zone_id: String,
    /// Unix timestamp (seconds) when the session was started.
    pub start_time: i64,
    pub rate: Rate,
    pub active: bool,
}

/// Manages the in-memory parking session (08-REQ-6.1 through 08-REQ-6.3).
pub struct Session {
    state: Option<SessionState>,
}

impl Session {
    /// Create a new empty session (no active session).
    pub fn new() -> Self {
        Session { state: None }
    }

    /// Returns `true` if a session is currently active.
    pub fn is_active(&self) -> bool {
        self.state.as_ref().map(|s| s.active).unwrap_or(false)
    }

    /// Start a new session, populating all fields (08-REQ-6.2).
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

    /// Stop the active session: set active=false and clear state (08-REQ-6.3).
    ///
    /// Returns the `session_id` if a session was active, `None` otherwise.
    pub fn stop(&mut self) -> Option<String> {
        let session_id = self.state.as_ref().map(|s| s.session_id.clone());
        self.state = None;
        session_id
    }

    /// Return the current session state, or `None` if no session is active.
    pub fn status(&self) -> Option<&SessionState> {
        self.state.as_ref()
    }

    /// Return the current rate, or `None` if no session is active.
    pub fn rate(&self) -> Option<&Rate> {
        self.state.as_ref().map(|s| &s.rate)
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rate(rate_type: &str, amount: f64, currency: &str) -> Rate {
        Rate {
            rate_type: rate_type.to_string(),
            amount,
            currency: currency.to_string(),
        }
    }

    /// TS-08-22: Session state fields are populated on start and cleared on stop.
    ///
    /// Requires: 08-REQ-6.1, 08-REQ-6.2, 08-REQ-6.3
    #[test]
    fn test_session_start_stop_fields() {
        let mut session = Session::new();
        assert!(!session.is_active(), "new session must be inactive");

        session.start(
            "s1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            make_rate("per_hour", 2.5, "EUR"),
        );

        let state = session.status().expect("status must be Some after start");
        assert_eq!(state.session_id, "s1");
        assert_eq!(state.zone_id, "zone-a");
        assert_eq!(state.start_time, 1_700_000_000);
        assert_eq!(state.rate.rate_type, "per_hour");
        assert_eq!(state.rate.amount, 2.5);
        assert_eq!(state.rate.currency, "EUR");
        assert!(state.active, "active must be true after start");

        session.stop();

        assert!(!session.is_active(), "session must be inactive after stop");
        assert!(
            session.status().is_none(),
            "status must be None after stop"
        );
    }

    /// TS-08-4: GetStatus returns active session details.
    ///
    /// Requires: 08-REQ-1.4
    #[test]
    fn test_get_status_active() {
        let mut session = Session::new();
        session.start(
            "sess-1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            make_rate("per_hour", 2.5, "EUR"),
        );

        let state = session.status().expect("must return Some when active");
        assert!(state.active, "active must be true");
        assert_eq!(state.session_id, "sess-1");
        assert_eq!(state.zone_id, "zone-a");
        assert_eq!(state.start_time, 1_700_000_000);
        assert_eq!(state.rate.rate_type, "per_hour");
    }

    /// TS-08-5: GetStatus returns None when no session is active.
    ///
    /// Requires: 08-REQ-1.4
    #[test]
    fn test_get_status_inactive() {
        let session = Session::new();
        assert!(!session.is_active(), "must be inactive");
        assert!(
            session.status().is_none(),
            "status must be None when no session"
        );
    }

    /// TS-08-6: GetRate returns the cached rate from the active session.
    ///
    /// Requires: 08-REQ-1.5
    #[test]
    fn test_get_rate_active() {
        let mut session = Session::new();
        session.start(
            "sess-1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            make_rate("flat_fee", 5.0, "EUR"),
        );

        let rate = session.rate().expect("rate must be Some when active");
        assert_eq!(rate.rate_type, "flat_fee");
        assert_eq!(rate.amount, 5.0);
        assert_eq!(rate.currency, "EUR");
    }

    /// TS-08-7: GetRate returns None when no session is active.
    ///
    /// Requires: 08-REQ-1.5
    #[test]
    fn test_get_rate_inactive() {
        let session = Session::new();
        assert!(
            session.rate().is_none(),
            "rate must be None when no session"
        );
    }
}
