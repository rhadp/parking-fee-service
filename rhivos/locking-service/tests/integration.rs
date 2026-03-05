//! Integration tests for LOCKING_SERVICE.
//!
//! These tests require a running Kuksa Databroker instance.
//! Run with: `cargo test -p locking-service --features integration`
//!
//! Gated behind the `integration` feature flag so they are skipped
//! during normal `cargo test` runs.

#![cfg(feature = "integration")]

/// TS-03-1: Lock command happy path.
///
/// Preconditions:
/// - DATA_BROKER running, Vehicle.Speed = 0.0, door closed.
///
/// Write a valid lock command -> verify IsLocked = true and success response.
#[test]
fn test_lock_command_happy_path() {
    // This test requires a running DATA_BROKER and the locking-service binary.
    // It will fail until the full implementation is complete.
    todo!(
        "Integration test: send lock command to DATA_BROKER, \
         verify IsLocked=true and success response"
    );
}

/// TS-03-2: Unlock command happy path.
///
/// Preconditions:
/// - DATA_BROKER running, Vehicle.Speed = 0.0, door closed, IsLocked = true.
///
/// Write unlock command -> verify IsLocked = false and success response.
#[test]
fn test_unlock_command_happy_path() {
    todo!(
        "Integration test: send unlock command to DATA_BROKER, \
         verify IsLocked=false and success response"
    );
}

/// TS-03-3: Safety constraint rejection -- vehicle moving.
///
/// Preconditions:
/// - DATA_BROKER running, Vehicle.Speed = 30.0, door closed.
///
/// Write lock command -> verify rejection with reason "vehicle_moving".
#[test]
fn test_safety_rejection_vehicle_moving() {
    todo!(
        "Integration test: set speed=30, send lock command, \
         verify 'vehicle_moving' failure response"
    );
}

/// TS-03-4: Safety constraint rejection -- door ajar.
///
/// Preconditions:
/// - DATA_BROKER running, Vehicle.Speed = 0.0, door open.
///
/// Write lock command -> verify rejection with reason "door_ajar".
#[test]
fn test_safety_rejection_door_ajar() {
    todo!(
        "Integration test: set door open, send lock command, \
         verify 'door_ajar' failure response"
    );
}

/// TS-03-E1: Invalid command JSON handling.
///
/// Write malformed JSON -> verify failure response with reason "invalid_command".
/// Then write valid command -> verify service still processes it.
#[test]
fn test_invalid_json_handling() {
    todo!(
        "Integration test: write malformed JSON, verify 'invalid_command' response, \
         then send valid command and verify it succeeds"
    );
}

/// TS-03-E2: Command with missing required fields.
///
/// Write JSON missing 'action' -> verify failure response with reason "invalid_command".
#[test]
fn test_missing_fields_handling() {
    todo!(
        "Integration test: write JSON missing required fields, \
         verify 'invalid_command' failure response"
    );
}

/// TS-03-E3: Command with invalid action value.
///
/// Write command with action="reboot" -> verify failure with reason "invalid_action".
#[test]
fn test_invalid_action_handling() {
    todo!(
        "Integration test: write command with action='reboot', \
         verify 'invalid_action' failure response"
    );
}
