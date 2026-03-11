/// Result of a safety constraint check.
///
/// Returns `Ok(())` if all safety constraints pass, or `Err(reason)` with
/// the specific constraint violated.
pub fn check_safety_constraints(_speed: Option<f64>, _door_open: Option<bool>) -> Result<(), String> {
    todo!("Implement safety constraint checks")
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- TS-03-P1, TS-03-P3: Safety constraint validation ---

    #[test]
    fn test_speed_zero_door_closed_passes() {
        // TS-03-P3: Speed == 0.0 and door closed passes safety check
        let result = check_safety_constraints(Some(0.0), Some(false));
        assert!(result.is_ok(), "Speed 0.0 and door closed should pass safety check");
    }

    #[test]
    fn test_speed_above_threshold_fails() {
        // TS-03-3: Speed >= 1.0 fails safety check with reason "vehicle_moving"
        let result = check_safety_constraints(Some(1.0), Some(false));
        assert!(result.is_err(), "Speed >= 1.0 should fail safety check");
        assert_eq!(result.unwrap_err(), "vehicle_moving");
    }

    #[test]
    fn test_speed_well_above_threshold_fails() {
        // TS-03-3: Speed well above threshold fails
        let result = check_safety_constraints(Some(30.0), Some(false));
        assert!(result.is_err(), "Speed 30.0 should fail safety check");
        assert_eq!(result.unwrap_err(), "vehicle_moving");
    }

    #[test]
    fn test_speed_below_threshold_passes() {
        // Speed just below 1.0 should pass
        let result = check_safety_constraints(Some(0.5), Some(false));
        assert!(result.is_ok(), "Speed 0.5 should pass safety check");
    }

    #[test]
    fn test_door_open_fails() {
        // TS-03-4: Door open == true fails safety check with reason "door_ajar"
        let result = check_safety_constraints(Some(0.0), Some(true));
        assert!(result.is_err(), "Door open should fail safety check");
        assert_eq!(result.unwrap_err(), "door_ajar");
    }

    #[test]
    fn test_both_constraints_violated_returns_speed_first() {
        // TS-03-P1: Both constraints violated returns speed failure first
        let result = check_safety_constraints(Some(50.0), Some(true));
        assert!(result.is_err(), "Both constraints violated should fail");
        assert_eq!(
            result.unwrap_err(),
            "vehicle_moving",
            "Speed should be checked first when both constraints are violated"
        );
    }

    #[test]
    fn test_no_speed_signal_defaults_to_safe() {
        // Design: If a safety signal has not been set, the check passes (safe default)
        let result = check_safety_constraints(None, Some(false));
        assert!(result.is_ok(), "Missing speed signal should default to safe (stationary)");
    }

    #[test]
    fn test_no_door_signal_defaults_to_safe() {
        // Design: If a safety signal has not been set, the check passes (safe default)
        let result = check_safety_constraints(Some(0.0), None);
        assert!(result.is_ok(), "Missing door signal should default to safe (door closed)");
    }

    #[test]
    fn test_both_signals_missing_defaults_to_safe() {
        // Design: Both signals missing should default to safe
        let result = check_safety_constraints(None, None);
        assert!(result.is_ok(), "Both signals missing should default to safe");
    }
}
