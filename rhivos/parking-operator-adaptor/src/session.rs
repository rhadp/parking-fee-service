//! Session and session state types.
//!
//! This module defines the Session struct and SessionState enum for
//! tracking parking session lifecycle.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::location::Location;

/// Session state enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SessionState {
    /// No active session
    #[default]
    None,
    /// Session is being started
    Starting,
    /// Session is active
    Active,
    /// Session is being stopped
    Stopping,
    /// Session has been stopped
    Stopped,
    /// Session encountered an error
    Error,
}


impl SessionState {
    /// Convert to proto SessionState value.
    pub fn to_proto(&self) -> i32 {
        match self {
            SessionState::None => 0,
            SessionState::Starting => 1,
            SessionState::Active => 2,
            SessionState::Stopping => 3,
            SessionState::Stopped => 4,
            SessionState::Error => 5,
        }
    }

    /// Create from proto SessionState value.
    pub fn from_proto(value: i32) -> Self {
        match value {
            1 => SessionState::Starting,
            2 => SessionState::Active,
            3 => SessionState::Stopping,
            4 => SessionState::Stopped,
            5 => SessionState::Error,
            _ => SessionState::None,
        }
    }
}

/// A parking session with all associated data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier from PARKING_OPERATOR
    pub session_id: String,
    /// Current session state
    pub state: SessionState,
    /// Session start time
    pub start_time: DateTime<Utc>,
    /// Session end time (set when stopped)
    pub end_time: Option<DateTime<Utc>>,
    /// Location where session started
    pub location: Location,
    /// Parking zone identifier
    pub zone_id: String,
    /// Hourly rate for this zone
    pub hourly_rate: f64,
    /// Current cost (updated periodically)
    pub current_cost: f64,
    /// Final cost (set when stopped)
    pub final_cost: Option<f64>,
    /// Error message if state is Error
    pub error_message: Option<String>,
    /// Last update timestamp
    pub last_updated: DateTime<Utc>,
}

impl Session {
    /// Create a new session in Starting state.
    pub fn new_starting(location: Location, zone_id: String) -> Self {
        let now = Utc::now();
        Self {
            session_id: String::new(),
            state: SessionState::Starting,
            start_time: now,
            end_time: None,
            location,
            zone_id,
            hourly_rate: 0.0,
            current_cost: 0.0,
            final_cost: None,
            error_message: None,
            last_updated: now,
        }
    }

    /// Get the duration of the session.
    pub fn duration(&self) -> Duration {
        let end = self.end_time.unwrap_or_else(Utc::now);
        let duration = end.signed_duration_since(self.start_time);
        Duration::from_secs(duration.num_seconds().max(0) as u64)
    }

    /// Get duration in seconds.
    pub fn duration_seconds(&self) -> i64 {
        self.duration().as_secs() as i64
    }

    /// Check if the session is active.
    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }

    /// Check if an operation is in progress (Starting or Stopping).
    pub fn is_in_progress(&self) -> bool {
        matches!(self.state, SessionState::Starting | SessionState::Stopping)
    }

    /// Transition to Active state.
    pub fn set_active(&mut self, session_id: String, hourly_rate: f64) {
        self.session_id = session_id;
        self.state = SessionState::Active;
        self.hourly_rate = hourly_rate;
        self.last_updated = Utc::now();
    }

    /// Transition to Stopping state.
    pub fn set_stopping(&mut self) {
        self.state = SessionState::Stopping;
        self.last_updated = Utc::now();
    }

    /// Transition to Stopped state.
    pub fn set_stopped(&mut self, final_cost: f64) {
        self.state = SessionState::Stopped;
        self.end_time = Some(Utc::now());
        self.final_cost = Some(final_cost);
        self.last_updated = Utc::now();
    }

    /// Transition to Error state.
    pub fn set_error(&mut self, message: String) {
        self.state = SessionState::Error;
        self.error_message = Some(message);
        self.last_updated = Utc::now();
    }

    /// Update current cost.
    pub fn update_cost(&mut self, cost: f64) {
        self.current_cost = cost;
        self.last_updated = Utc::now();
    }

    /// Get start time as Unix timestamp.
    pub fn start_time_unix(&self) -> i64 {
        self.start_time.timestamp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_session_state_default() {
        assert_eq!(SessionState::default(), SessionState::None);
    }

    #[test]
    fn test_session_state_proto_roundtrip() {
        let states = [
            SessionState::None,
            SessionState::Starting,
            SessionState::Active,
            SessionState::Stopping,
            SessionState::Stopped,
            SessionState::Error,
        ];

        for state in states {
            let proto = state.to_proto();
            let back = SessionState::from_proto(proto);
            assert_eq!(state, back);
        }
    }

    #[test]
    fn test_session_new_starting() {
        let location = Location::new(37.7749, -122.4194);
        let session = Session::new_starting(location.clone(), "zone-1".to_string());

        assert_eq!(session.state, SessionState::Starting);
        assert!(session.session_id.is_empty());
        assert_eq!(session.zone_id, "zone-1");
    }

    #[test]
    fn test_session_transitions() {
        let location = Location::new(37.7749, -122.4194);
        let mut session = Session::new_starting(location, "zone-1".to_string());

        // Starting -> Active
        session.set_active("session-123".to_string(), 2.50);
        assert_eq!(session.state, SessionState::Active);
        assert_eq!(session.session_id, "session-123");
        assert!((session.hourly_rate - 2.50).abs() < 0.01);

        // Active -> Stopping
        session.set_stopping();
        assert_eq!(session.state, SessionState::Stopping);

        // Stopping -> Stopped
        session.set_stopped(5.00);
        assert_eq!(session.state, SessionState::Stopped);
        assert!(session.end_time.is_some());
        assert_eq!(session.final_cost, Some(5.00));
    }

    #[test]
    fn test_session_error() {
        let location = Location::new(37.7749, -122.4194);
        let mut session = Session::new_starting(location, "zone-1".to_string());

        session.set_error("API failed".to_string());
        assert_eq!(session.state, SessionState::Error);
        assert_eq!(session.error_message, Some("API failed".to_string()));
    }

    #[test]
    fn test_session_is_active() {
        let location = Location::new(37.7749, -122.4194);
        let mut session = Session::new_starting(location, "zone-1".to_string());

        assert!(!session.is_active());
        session.set_active("session-123".to_string(), 2.50);
        assert!(session.is_active());
    }

    #[test]
    fn test_session_is_in_progress() {
        let location = Location::new(37.7749, -122.4194);
        let mut session = Session::new_starting(location, "zone-1".to_string());

        assert!(session.is_in_progress());
        session.set_active("session-123".to_string(), 2.50);
        assert!(!session.is_in_progress());
        session.set_stopping();
        assert!(session.is_in_progress());
    }

    // Property 12: State Transition Timestamp Recording
    // Validates: Requirements 7.1, 7.2
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_state_transition_updates_timestamp(
            lat in -90.0f64..90.0,
            lng in -180.0f64..180.0
        ) {
            let location = Location::new(lat, lng);
            let mut session = Session::new_starting(location, "zone-1".to_string());
            let initial_time = session.last_updated;

            // Small delay to ensure timestamp changes
            std::thread::sleep(std::time::Duration::from_millis(1));

            session.set_active("session-123".to_string(), 2.50);
            prop_assert!(session.last_updated > initial_time);

            let prev_time = session.last_updated;
            std::thread::sleep(std::time::Duration::from_millis(1));

            session.set_stopping();
            prop_assert!(session.last_updated > prev_time);
        }
    }
}
