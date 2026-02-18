//! Parking session state management.
//!
//! This module defines the [`ParkingSession`] struct that tracks the state
//! of an active or completed parking session. It is held behind an
//! `Arc<Mutex<Option<ParkingSession>>>` to allow safe concurrent access from
//! the lock watcher and gRPC server tasks.
//!
//! # Requirements
//!
//! - 04-REQ-1.2, 04-REQ-1.4: Session state transitions on lock/unlock events.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Rate type used for fee calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateType {
    /// Fee = rate_amount x ceil(duration_minutes).
    PerMinute,
    /// Fee = fixed rate_amount regardless of duration.
    Flat,
}

impl fmt::Display for RateType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RateType::PerMinute => write!(f, "per_minute"),
            RateType::Flat => write!(f, "flat"),
        }
    }
}

impl RateType {
    /// Parse a rate type from a string (e.g. from JSON).
    pub fn from_str_loose(s: &str) -> Self {
        match s {
            "per_minute" => RateType::PerMinute,
            "flat" => RateType::Flat,
            _ => RateType::PerMinute, // default
        }
    }
}

/// Session status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Session is currently active.
    Active,
    /// Session has been completed.
    Completed,
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionStatus::Active => write!(f, "active"),
            SessionStatus::Completed => write!(f, "completed"),
        }
    }
}

/// A parking session tracking state, timing, and fee information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParkingSession {
    /// Unique session identifier (assigned by the parking operator).
    pub session_id: String,
    /// Vehicle identification number.
    pub vehicle_id: String,
    /// Parking zone identifier.
    pub zone_id: String,
    /// Unix timestamp when the session started.
    pub start_time: i64,
    /// Unix timestamp when the session ended (if completed).
    pub end_time: Option<i64>,
    /// Rate type for fee calculation.
    pub rate_type: RateType,
    /// Rate amount per unit (per minute or flat).
    pub rate_amount: f64,
    /// Currency code (e.g. "EUR").
    pub currency: String,
    /// Total fee (set when session is completed).
    pub total_fee: Option<f64>,
    /// Session status.
    pub status: SessionStatus,
}

impl ParkingSession {
    /// Returns `true` if the session is currently active.
    pub fn is_active(&self) -> bool {
        self.status == SessionStatus::Active
    }

    /// Calculate the current fee for an active session based on elapsed time.
    ///
    /// For per-minute rates: `rate_amount x ceil(elapsed_minutes)`.
    /// For flat rates: fixed `rate_amount`.
    /// Returns 0.0 if the session is completed (use `total_fee` instead).
    pub fn current_fee(&self, now: i64) -> f64 {
        if !self.is_active() {
            return self.total_fee.unwrap_or(0.0);
        }

        let elapsed_seconds = (now - self.start_time).max(0);
        calculate_fee(&self.rate_type, self.rate_amount, elapsed_seconds)
    }

    /// Mark the session as completed with the stop response data.
    pub fn complete(&mut self, end_time: i64, total_fee: f64, duration_seconds: i64) {
        self.end_time = Some(end_time);
        self.total_fee = Some(total_fee);
        self.status = SessionStatus::Completed;
        // Recalculate end_time from duration if not already set
        if self.end_time.is_none() {
            self.end_time = Some(self.start_time + duration_seconds);
        }
    }

    /// Duration in seconds. For active sessions, uses `now` as end time.
    pub fn duration_seconds(&self, now: i64) -> i64 {
        let end = self.end_time.unwrap_or(now);
        (end - self.start_time).max(0)
    }
}

/// Calculate a parking fee based on rate type and duration.
///
/// For `PerMinute`: `rate_amount x ceil(duration_seconds / 60)`.
/// For `Flat`: fixed `rate_amount` regardless of duration.
pub fn calculate_fee(rate_type: &RateType, rate_amount: f64, duration_seconds: i64) -> f64 {
    match rate_type {
        RateType::PerMinute => {
            let minutes = (duration_seconds as f64 / 60.0).ceil();
            rate_amount * minutes
        }
        RateType::Flat => rate_amount,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_session() -> ParkingSession {
        ParkingSession {
            session_id: "sess-001".into(),
            vehicle_id: "DEMO0000000000001".into(),
            zone_id: "zone-1".into(),
            start_time: 1_708_300_800,
            end_time: None,
            rate_type: RateType::PerMinute,
            rate_amount: 0.05,
            currency: "EUR".into(),
            total_fee: None,
            status: SessionStatus::Active,
        }
    }

    // ── RateType ───────────────────────────────────────────────────────────

    #[test]
    fn rate_type_display() {
        assert_eq!(RateType::PerMinute.to_string(), "per_minute");
        assert_eq!(RateType::Flat.to_string(), "flat");
    }

    #[test]
    fn rate_type_from_str_loose() {
        assert_eq!(RateType::from_str_loose("per_minute"), RateType::PerMinute);
        assert_eq!(RateType::from_str_loose("flat"), RateType::Flat);
        assert_eq!(RateType::from_str_loose("unknown"), RateType::PerMinute);
    }

    #[test]
    fn rate_type_serde_round_trip() {
        let json = serde_json::to_string(&RateType::PerMinute).unwrap();
        assert_eq!(json, "\"per_minute\"");
        let parsed: RateType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, RateType::PerMinute);

        let json = serde_json::to_string(&RateType::Flat).unwrap();
        assert_eq!(json, "\"flat\"");
        let parsed: RateType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, RateType::Flat);
    }

    // ── SessionStatus ──────────────────────────────────────────────────────

    #[test]
    fn session_status_display() {
        assert_eq!(SessionStatus::Active.to_string(), "active");
        assert_eq!(SessionStatus::Completed.to_string(), "completed");
    }

    // ── ParkingSession ─────────────────────────────────────────────────────

    #[test]
    fn new_session_is_active() {
        let session = sample_session();
        assert!(session.is_active());
        assert_eq!(session.status, SessionStatus::Active);
        assert!(session.end_time.is_none());
        assert!(session.total_fee.is_none());
    }

    #[test]
    fn complete_marks_session_completed() {
        let mut session = sample_session();
        let end_time = session.start_time + 300; // 5 minutes later
        session.complete(end_time, 0.25, 300);

        assert!(!session.is_active());
        assert_eq!(session.status, SessionStatus::Completed);
        assert_eq!(session.end_time, Some(end_time));
        assert_eq!(session.total_fee, Some(0.25));
    }

    #[test]
    fn current_fee_per_minute_active() {
        let session = sample_session();
        // 5 minutes later
        let now = session.start_time + 300;
        let fee = session.current_fee(now);
        // 0.05 * ceil(300/60) = 0.05 * 5 = 0.25
        assert!((fee - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn current_fee_per_minute_partial_minute() {
        let session = sample_session();
        // 3 minutes and 30 seconds later
        let now = session.start_time + 210;
        let fee = session.current_fee(now);
        // 0.05 * ceil(210/60) = 0.05 * ceil(3.5) = 0.05 * 4 = 0.20
        assert!((fee - 0.20).abs() < f64::EPSILON);
    }

    #[test]
    fn current_fee_flat_rate() {
        let mut session = sample_session();
        session.rate_type = RateType::Flat;
        session.rate_amount = 5.00;

        // Flat rate: fee is always the rate_amount
        let now = session.start_time + 3600; // 1 hour later
        let fee = session.current_fee(now);
        assert!((fee - 5.00).abs() < f64::EPSILON);
    }

    #[test]
    fn current_fee_completed_returns_total() {
        let mut session = sample_session();
        session.complete(session.start_time + 300, 0.25, 300);

        let now = session.start_time + 600; // well after completion
        let fee = session.current_fee(now);
        assert!((fee - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn current_fee_zero_elapsed() {
        let session = sample_session();
        let fee = session.current_fee(session.start_time);
        // ceil(0/60) = 0
        assert!((fee - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn current_fee_negative_elapsed_clamped() {
        let session = sample_session();
        // Time before start (shouldn't happen, but should be safe)
        let fee = session.current_fee(session.start_time - 100);
        assert!((fee - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn duration_seconds_active() {
        let session = sample_session();
        let now = session.start_time + 600;
        assert_eq!(session.duration_seconds(now), 600);
    }

    #[test]
    fn duration_seconds_completed() {
        let mut session = sample_session();
        let end = session.start_time + 300;
        session.complete(end, 0.25, 300);
        // Once completed, duration is fixed regardless of `now`
        let now = session.start_time + 9999;
        assert_eq!(session.duration_seconds(now), 300);
    }

    // ── calculate_fee ──────────────────────────────────────────────────────

    #[test]
    fn calculate_fee_per_minute_exact_minutes() {
        let fee = calculate_fee(&RateType::PerMinute, 0.05, 300);
        assert!((fee - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn calculate_fee_per_minute_partial_minutes() {
        let fee = calculate_fee(&RateType::PerMinute, 0.05, 301);
        // ceil(301/60) = ceil(5.016...) = 6
        assert!((fee - 0.30).abs() < f64::EPSILON);
    }

    #[test]
    fn calculate_fee_per_minute_zero_duration() {
        let fee = calculate_fee(&RateType::PerMinute, 0.05, 0);
        assert!((fee - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn calculate_fee_per_minute_one_second() {
        let fee = calculate_fee(&RateType::PerMinute, 0.05, 1);
        // ceil(1/60) = 1
        assert!((fee - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn calculate_fee_flat_rate() {
        let fee = calculate_fee(&RateType::Flat, 5.00, 3600);
        assert!((fee - 5.00).abs() < f64::EPSILON);
    }

    #[test]
    fn calculate_fee_flat_rate_zero_duration() {
        let fee = calculate_fee(&RateType::Flat, 5.00, 0);
        assert!((fee - 5.00).abs() < f64::EPSILON);
    }

    // ── Serde round-trip ───────────────────────────────────────────────────

    #[test]
    fn session_serde_round_trip() {
        let session = sample_session();
        let json = serde_json::to_string(&session).unwrap();
        let parsed: ParkingSession = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, session);
    }

    #[test]
    fn completed_session_serde_round_trip() {
        let mut session = sample_session();
        session.complete(session.start_time + 300, 0.25, 300);
        let json = serde_json::to_string(&session).unwrap();
        let parsed: ParkingSession = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, session);
    }
}
