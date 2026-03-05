/// Check safety constraints before executing a lock/unlock command.
///
/// Reads vehicle speed and door state and returns:
/// - `Ok(())` if all safety constraints are satisfied
/// - `Err(reason)` with the specific constraint violated
///
/// Speed is checked first; if speed >= 1.0 km/h, returns `Err("vehicle_moving")`.
/// Then door state is checked; if door is open, returns `Err("door_ajar")`.
///
/// If a signal value is not available, the safe default is assumed
/// (speed = 0, door closed).
pub fn check_safety_constraints(
    _speed: Option<f64>,
    _door_open: Option<bool>,
) -> Result<(), String> {
    todo!("Implement safety constraint checks")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-03-P1, TS-03-P3: speed=0.0, door closed -> safe.
    #[test]
    fn test_speed_zero_door_closed_passes() {
        let result = check_safety_constraints(Some(0.0), Some(false));
        assert!(result.is_ok(), "Speed 0 and door closed should pass safety check");
    }

    /// TS-03-P1: speed >= 1.0 -> vehicle_moving.
    #[test]
    fn test_speed_above_threshold_fails() {
        let result = check_safety_constraints(Some(1.0), Some(false));
        assert!(result.is_err(), "Speed >= 1.0 should fail safety check");
        assert_eq!(result.unwrap_err(), "vehicle_moving");
    }

    /// TS-03-P1: speed=30.0 -> vehicle_moving.
    #[test]
    fn test_high_speed_fails() {
        let result = check_safety_constraints(Some(30.0), Some(false));
        assert!(result.is_err(), "High speed should fail safety check");
        assert_eq!(result.unwrap_err(), "vehicle_moving");
    }

    /// TS-03-P1: door open -> door_ajar.
    #[test]
    fn test_door_open_fails() {
        let result = check_safety_constraints(Some(0.0), Some(true));
        assert!(result.is_err(), "Door open should fail safety check");
        assert_eq!(result.unwrap_err(), "door_ajar");
    }

    /// TS-03-P1: both constraints violated -> speed checked first.
    #[test]
    fn test_both_violated_returns_speed_first() {
        let result = check_safety_constraints(Some(5.0), Some(true));
        assert!(result.is_err(), "Both violated should fail");
        assert_eq!(
            result.unwrap_err(),
            "vehicle_moving",
            "Speed should be checked first"
        );
    }

    /// Design: no signal value -> safe default (stationary, door closed).
    #[test]
    fn test_no_signal_values_passes() {
        let result = check_safety_constraints(None, None);
        assert!(result.is_ok(), "No signal values should default to safe");
    }

    /// Speed below threshold (0.5) should pass.
    #[test]
    fn test_speed_below_threshold_passes() {
        let result = check_safety_constraints(Some(0.5), Some(false));
        assert!(result.is_ok(), "Speed below 1.0 should pass safety check");
    }

    /// Speed exactly 0.99 should pass (boundary test).
    #[test]
    fn test_speed_just_below_threshold_passes() {
        let result = check_safety_constraints(Some(0.99), Some(false));
        assert!(result.is_ok(), "Speed 0.99 should pass safety check");
    }
}
