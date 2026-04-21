//! In-memory parking session state management.
//!
//! Maintains the current session record and provides typed accessors.

use serde::{Deserialize, Serialize};

/// Parking rate returned by the PARKING_OPERATOR.
///
/// NOTE: The JSON key for `rate_type` is `"type"` — serde rename required
/// in deserialization contexts: `#[serde(rename = "type")]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rate {
    /// Rate type: `"per_hour"` or `"flat_fee"`.
    #[serde(rename = "type")]
    pub rate_type: String,
    /// Numeric rate amount.
    pub amount: f64,
    /// ISO 4217 currency code, e.g. `"EUR"`.
    pub currency: String,
}

/// Snapshot of an active parking session.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionState {
    pub session_id: String,
    pub zone_id: String,
    /// Unix timestamp (seconds) when the session started.
    pub start_time: i64,
    pub rate: Rate,
    pub active: bool,
}

/// In-memory session manager.
///
/// Wraps an `Option<SessionState>` and provides start/stop/query operations.
#[allow(dead_code)] // field used once implementation is complete (task group 2)
#[derive(Debug, Default)]
pub struct Session {
    inner: Option<SessionState>,
}

impl Session {
    /// Create a new session with no active record.
    pub fn new() -> Self {
        Session { inner: None }
    }

    /// Returns `true` if a session is currently active.
    pub fn is_active(&self) -> bool {
        todo!("implement Session::is_active")
    }

    /// Start a new session, populating all state fields.
    ///
    /// # Panics
    /// (implementation) — panics with `todo!()` until implemented.
    pub fn start(
        &mut self,
        _session_id: String,
        _zone_id: String,
        _start_time: i64,
        _rate: Rate,
    ) {
        todo!("implement Session::start")
    }

    /// Stop the active session, clearing all state.
    ///
    /// Returns the `session_id` of the stopped session, or `None` if no
    /// session was active.
    pub fn stop(&mut self) -> Option<String> {
        todo!("implement Session::stop")
    }

    /// Returns a reference to the current session state, or `None`.
    pub fn status(&self) -> Option<&SessionState> {
        todo!("implement Session::status")
    }

    /// Returns a reference to the current session's rate, or `None`.
    pub fn rate(&self) -> Option<&Rate> {
        todo!("implement Session::rate")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rate() -> Rate {
        Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.50,
            currency: "EUR".to_string(),
        }
    }

    /// TS-08-22: Session state fields are populated on start, cleared on stop.
    ///
    /// Verifies: 08-REQ-6.1, 08-REQ-6.2, 08-REQ-6.3
    #[test]
    fn test_session_start_stop_fields() {
        let mut session = Session::new();
        assert!(!session.is_active());

        session.start("s1".to_string(), "zone-a".to_string(), 1_700_000_000, make_rate());

        let state = session.status().expect("session should be active");
        assert_eq!(state.session_id, "s1");
        assert_eq!(state.zone_id, "zone-a");
        assert_eq!(state.start_time, 1_700_000_000);
        assert_eq!(state.rate.rate_type, "per_hour");
        assert!(state.active);

        session.stop();
        assert!(!session.is_active());
        assert!(session.status().is_none());
    }

    /// TS-08-4: GetStatus returns full details when a session is active.
    ///
    /// Verifies: 08-REQ-1.4
    #[test]
    fn test_get_status_active() {
        let mut session = Session::new();
        session.start(
            "sess-1".to_string(),
            "zone-a".to_string(),
            1_700_000_000,
            make_rate(),
        );

        assert!(session.is_active());
        let state = session.status().expect("session must be active");
        assert_eq!(state.session_id, "sess-1");
        assert_eq!(state.zone_id, "zone-a");
        assert_eq!(state.start_time, 1_700_000_000);
        assert_eq!(state.rate.rate_type, "per_hour");
    }

    /// TS-08-5: GetStatus returns active=false with empty fields when no session.
    ///
    /// Verifies: 08-REQ-1.4
    #[test]
    fn test_get_status_inactive() {
        let session = Session::new();

        assert!(!session.is_active());
        assert!(session.status().is_none());
    }

    /// TS-08-6: GetRate returns cached rate from the active session.
    ///
    /// Verifies: 08-REQ-1.5
    #[test]
    fn test_get_rate_active() {
        let mut session = Session::new();
        let rate = Rate {
            rate_type: "flat_fee".to_string(),
            amount: 5.00,
            currency: "EUR".to_string(),
        };
        session.start("sess-1".to_string(), "zone-a".to_string(), 0, rate.clone());

        let cached = session.rate().expect("rate must be present");
        assert_eq!(cached.rate_type, "flat_fee");
        assert!((cached.amount - 5.00).abs() < f64::EPSILON);
        assert_eq!(cached.currency, "EUR");
    }

    /// TS-08-7: GetRate returns empty (None) when no session is active.
    ///
    /// Verifies: 08-REQ-1.5
    #[test]
    fn test_get_rate_inactive() {
        let session = Session::new();

        assert!(session.rate().is_none());
    }
}
