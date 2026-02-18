//! Safety validation logic for lock/unlock commands.
//!
//! This module contains pure functions that determine whether a lock or unlock
//! command is safe to execute, based on the current vehicle state (speed and
//! door position). These functions have no side effects and are fully
//! deterministic for a given set of inputs.
//!
//! # Safety Rules
//!
//! - **Speed check (lock AND unlock):** If `Vehicle.Speed >= max_speed_kmh`,
//!   the command is rejected with `RejectedSpeed`.
//! - **Door-ajar check (lock only):** If the driver-side door is open and the
//!   command is to lock, the command is rejected with `RejectedDoorOpen`.
//! - **Unlock with door open:** Allowed — there is no door-ajar constraint for
//!   unlocking.
//!
//! # Requirements
//!
//! - 02-REQ-3.1: Lock rejected when speed >= 1.0 km/h
//! - 02-REQ-3.2: Lock rejected when door is open
//! - 02-REQ-3.3: Unlock rejected when speed >= 1.0 km/h
//! - 02-REQ-3.4: Unlock does NOT check door-ajar

use std::fmt;

/// The outcome of a lock/unlock safety validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockResult {
    /// The command passed all safety checks and may be executed.
    Success,
    /// The command was rejected because the vehicle speed is too high.
    RejectedSpeed,
    /// The command was rejected because the driver-side door is open (lock only).
    RejectedDoorOpen,
}

impl fmt::Display for LockResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockResult::Success => write!(f, "SUCCESS"),
            LockResult::RejectedSpeed => write!(f, "REJECTED_SPEED"),
            LockResult::RejectedDoorOpen => write!(f, "REJECTED_DOOR_OPEN"),
        }
    }
}

/// Validate whether a lock/unlock command is safe to execute.
///
/// This is a pure function with no side effects. The result depends only on the
/// provided inputs and the speed threshold.
///
/// # Arguments
///
/// * `command_is_lock` - `true` for a lock command, `false` for an unlock command.
/// * `speed_kmh` - Current vehicle speed in km/h.
/// * `door_is_open` - Whether the driver-side door is currently open.
/// * `max_speed_kmh` - Maximum allowed speed for lock/unlock operations.
///
/// # Returns
///
/// A [`LockResult`] indicating whether the command is safe to execute or the
/// reason for rejection.
///
/// # Decision Rules
///
/// 1. If `speed_kmh >= max_speed_kmh`, return `RejectedSpeed` (applies to both
///    lock and unlock).
/// 2. If `command_is_lock` is `true` AND `door_is_open` is `true`, return
///    `RejectedDoorOpen`.
/// 3. Otherwise, return `Success`.
pub fn validate_lock(
    command_is_lock: bool,
    speed_kmh: f32,
    door_is_open: bool,
    max_speed_kmh: f32,
) -> LockResult {
    if speed_kmh >= max_speed_kmh {
        return LockResult::RejectedSpeed;
    }
    if command_is_lock && door_is_open {
        return LockResult::RejectedDoorOpen;
    }
    LockResult::Success
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Display trait tests ─────────────────────────────────────────────

    #[test]
    fn display_success() {
        assert_eq!(LockResult::Success.to_string(), "SUCCESS");
    }

    #[test]
    fn display_rejected_speed() {
        assert_eq!(LockResult::RejectedSpeed.to_string(), "REJECTED_SPEED");
    }

    #[test]
    fn display_rejected_door_open() {
        assert_eq!(
            LockResult::RejectedDoorOpen.to_string(),
            "REJECTED_DOOR_OPEN"
        );
    }

    // ── Deterministic boundary-value tests ──────────────────────────────

    /// Lock with safe conditions (speed = 0.0, door closed) => SUCCESS
    #[test]
    fn lock_safe_conditions() {
        let result = validate_lock(true, 0.0, false, 1.0);
        assert_eq!(result, LockResult::Success);
    }

    /// Unlock with safe conditions (speed = 0.0, door closed) => SUCCESS
    #[test]
    fn unlock_safe_conditions() {
        let result = validate_lock(false, 0.0, false, 1.0);
        assert_eq!(result, LockResult::Success);
    }

    /// Lock at exactly the speed threshold (1.0 km/h) => REJECTED_SPEED
    /// Requirement 02-REQ-3.1: speed >= 1.0 rejects.
    #[test]
    fn lock_at_speed_threshold() {
        let result = validate_lock(true, 1.0, false, 1.0);
        assert_eq!(result, LockResult::RejectedSpeed);
    }

    /// Unlock at exactly the speed threshold => REJECTED_SPEED
    /// Requirement 02-REQ-3.3: unlock also checks speed.
    #[test]
    fn unlock_at_speed_threshold() {
        let result = validate_lock(false, 1.0, false, 1.0);
        assert_eq!(result, LockResult::RejectedSpeed);
    }

    /// Lock just below speed threshold (0.99 km/h) => SUCCESS
    #[test]
    fn lock_just_below_speed_threshold() {
        let result = validate_lock(true, 0.99, false, 1.0);
        assert_eq!(result, LockResult::Success);
    }

    /// Lock well above speed threshold (50.0 km/h) => REJECTED_SPEED
    #[test]
    fn lock_high_speed() {
        let result = validate_lock(true, 50.0, false, 1.0);
        assert_eq!(result, LockResult::RejectedSpeed);
    }

    /// Lock with door open and safe speed => REJECTED_DOOR_OPEN
    /// Requirement 02-REQ-3.2: lock rejected when door is open.
    #[test]
    fn lock_door_open() {
        let result = validate_lock(true, 0.0, true, 1.0);
        assert_eq!(result, LockResult::RejectedDoorOpen);
    }

    /// Lock with door open AND high speed => REJECTED_SPEED (speed check first)
    #[test]
    fn lock_door_open_and_high_speed() {
        let result = validate_lock(true, 5.0, true, 1.0);
        assert_eq!(result, LockResult::RejectedSpeed);
    }

    /// Unlock with door open and safe speed => SUCCESS
    /// Requirement 02-REQ-3.4: unlock does NOT check door-ajar.
    #[test]
    fn unlock_door_open_safe_speed() {
        let result = validate_lock(false, 0.0, true, 1.0);
        assert_eq!(result, LockResult::Success);
    }

    /// Unlock with door open and high speed => REJECTED_SPEED
    #[test]
    fn unlock_door_open_high_speed() {
        let result = validate_lock(false, 5.0, true, 1.0);
        assert_eq!(result, LockResult::RejectedSpeed);
    }

    // ── Eq / Clone / Copy trait tests ───────────────────────────────────

    #[test]
    fn lock_result_is_clone() {
        let result = LockResult::Success;
        let cloned = result.clone();
        assert_eq!(result, cloned);
    }

    #[test]
    fn lock_result_is_copy() {
        let result = LockResult::RejectedSpeed;
        let copied = result;
        // Both still usable (Copy trait).
        assert_eq!(result, copied);
    }

    #[test]
    fn lock_result_debug() {
        let debug_str = format!("{:?}", LockResult::RejectedDoorOpen);
        assert_eq!(debug_str, "RejectedDoorOpen");
    }

    // ── Custom threshold tests ──────────────────────────────────────────

    /// Verify the threshold is configurable (not hardcoded to 1.0).
    #[test]
    fn custom_threshold_5kmh() {
        // Speed 3.0 with threshold 5.0 => allowed.
        assert_eq!(validate_lock(true, 3.0, false, 5.0), LockResult::Success);
        // Speed 5.0 with threshold 5.0 => rejected.
        assert_eq!(
            validate_lock(true, 5.0, false, 5.0),
            LockResult::RejectedSpeed
        );
    }

    // ── Property-based tests (proptest) ─────────────────────────────────
    //
    // These validate **Property 4: Safety Function Purity** — the function
    // is deterministic and depends only on its inputs.
    //
    // Requirements validated:
    //   02-REQ-3.1: Lock rejected when speed >= threshold
    //   02-REQ-3.2: Lock rejected when door is open
    //   02-REQ-3.3: Unlock rejected when speed >= threshold
    //   02-REQ-3.4: Unlock does NOT check door-ajar

    mod prop {
        use super::*;
        use proptest::prelude::*;

        /// Speed range: 0.0 to 300.0 km/h (reasonable vehicle speed range).
        fn speed_strategy() -> impl Strategy<Value = f32> {
            0.0_f32..300.0_f32
        }

        /// Threshold range: 0.1 to 100.0 km/h (reasonable threshold range).
        fn threshold_strategy() -> impl Strategy<Value = f32> {
            0.1_f32..100.0_f32
        }

        // ── Rule 1: Speed >= threshold => RejectedSpeed ─────────────────
        // Applies to BOTH lock and unlock commands (02-REQ-3.1, 02-REQ-3.3).

        proptest! {
            #[test]
            fn prop_speed_above_threshold_always_rejected(
                command_is_lock in proptest::bool::ANY,
                door_is_open in proptest::bool::ANY,
                threshold in threshold_strategy(),
                // Speed at or above threshold
                speed_offset in 0.0_f32..200.0_f32,
            ) {
                let speed = threshold + speed_offset;
                let result = validate_lock(command_is_lock, speed, door_is_open, threshold);
                prop_assert_eq!(result, LockResult::RejectedSpeed);
            }
        }

        // ── Rule 2: Lock + door open + speed below threshold => RejectedDoorOpen
        // (02-REQ-3.2)

        proptest! {
            #[test]
            fn prop_lock_with_door_open_below_threshold_rejected(
                threshold in threshold_strategy(),
                // Speed strictly below threshold
                speed_ratio in 0.0_f32..1.0_f32,
            ) {
                let speed = threshold * speed_ratio * 0.999; // ensure < threshold
                let result = validate_lock(true, speed, true, threshold);
                prop_assert_eq!(result, LockResult::RejectedDoorOpen);
            }
        }

        // ── Rule 3: Lock + door closed + speed below threshold => Success

        proptest! {
            #[test]
            fn prop_lock_safe_conditions_success(
                threshold in threshold_strategy(),
                speed_ratio in 0.0_f32..1.0_f32,
            ) {
                let speed = threshold * speed_ratio * 0.999;
                let result = validate_lock(true, speed, false, threshold);
                prop_assert_eq!(result, LockResult::Success);
            }
        }

        // ── Rule 4: Unlock + speed below threshold => Success (regardless of door)
        // (02-REQ-3.4)

        proptest! {
            #[test]
            fn prop_unlock_below_threshold_always_success(
                door_is_open in proptest::bool::ANY,
                threshold in threshold_strategy(),
                speed_ratio in 0.0_f32..1.0_f32,
            ) {
                let speed = threshold * speed_ratio * 0.999;
                let result = validate_lock(false, speed, door_is_open, threshold);
                prop_assert_eq!(result, LockResult::Success);
            }
        }

        // ── Purity: same inputs always produce the same output ──────────

        proptest! {
            #[test]
            fn prop_deterministic(
                command_is_lock in proptest::bool::ANY,
                speed in speed_strategy(),
                door_is_open in proptest::bool::ANY,
                threshold in threshold_strategy(),
            ) {
                let result1 = validate_lock(command_is_lock, speed, door_is_open, threshold);
                let result2 = validate_lock(command_is_lock, speed, door_is_open, threshold);
                prop_assert_eq!(result1, result2);
            }
        }

        // ── Exhaustive coverage: every result matches exactly one rule ──

        proptest! {
            #[test]
            fn prop_result_matches_decision_rules(
                command_is_lock in proptest::bool::ANY,
                speed in speed_strategy(),
                door_is_open in proptest::bool::ANY,
                threshold in threshold_strategy(),
            ) {
                let result = validate_lock(command_is_lock, speed, door_is_open, threshold);

                let expected = if speed >= threshold {
                    LockResult::RejectedSpeed
                } else if command_is_lock && door_is_open {
                    LockResult::RejectedDoorOpen
                } else {
                    LockResult::Success
                };

                prop_assert_eq!(result, expected);
            }
        }

        // ── Speed exactly at threshold (boundary) ───────────────────────

        proptest! {
            #[test]
            fn prop_speed_at_exact_threshold_rejected(
                command_is_lock in proptest::bool::ANY,
                door_is_open in proptest::bool::ANY,
                threshold in threshold_strategy(),
            ) {
                let result = validate_lock(command_is_lock, threshold, door_is_open, threshold);
                prop_assert_eq!(result, LockResult::RejectedSpeed);
            }
        }
    }
}
