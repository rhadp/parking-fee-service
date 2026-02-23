//! LOCKING_SERVICE integration tests
//!
//! These tests verify LOCKING_SERVICE command subscription, lock/unlock
//! execution, and safety constraint enforcement.
//!
//! Test Spec: TS-02-6, TS-02-8, TS-02-9, TS-02-10, TS-02-11, TS-02-12,
//!            TS-02-13, TS-02-14

/// Helper: check if infrastructure is available.
fn infra_available() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:55556".parse().unwrap(),
        std::time::Duration::from_secs(2),
    )
    .is_ok()
}

macro_rules! require_infra {
    () => {
        if !infra_available() {
            eprintln!("SKIP: infrastructure not available (run `make infra-up`)");
            return;
        }
    };
}

/// TS-02-6: LOCKING_SERVICE subscribes to command signals (02-REQ-2.1)
///
/// Verify LOCKING_SERVICE subscribes to Vehicle.Command.Door.Lock from
/// DATA_BROKER via UDS. When a command is written to the signal, the service
/// should process it and write a response.
#[test]
#[ignore = "requires DATA_BROKER + LOCKING_SERVICE running"]
fn test_locking_subscribes_to_commands() {
    require_infra!();

    // Steps:
    // 1. Write a lock command to Vehicle.Command.Door.Lock via DATA_BROKER
    // 2. Wait for Vehicle.Command.Door.Response to appear
    // 3. Verify response contains the correct command_id
    panic!("not implemented: requires running LOCKING_SERVICE and databroker-client");
}

/// TS-02-8: LOCKING_SERVICE executes lock action (02-REQ-2.3)
///
/// Verify LOCKING_SERVICE locks doors when action is "lock" and safety
/// constraints are met (speed = 0, door closed).
#[test]
#[ignore = "requires DATA_BROKER + LOCKING_SERVICE running"]
fn test_locking_executes_lock() {
    require_infra!();

    // Steps:
    // 1. Set Vehicle.Speed = 0.0
    // 2. Set IsOpen = false
    // 3. Send lock command with action "lock"
    // 4. Wait for response
    // 5. Verify IsLocked becomes true
    panic!("not implemented: requires running LOCKING_SERVICE and databroker-client");
}

/// TS-02-9: LOCKING_SERVICE executes unlock action (02-REQ-2.4)
///
/// Verify LOCKING_SERVICE unlocks doors when action is "unlock" and safety
/// constraints are met (speed = 0).
#[test]
#[ignore = "requires DATA_BROKER + LOCKING_SERVICE running"]
fn test_locking_executes_unlock() {
    require_infra!();

    // Steps:
    // 1. Set Vehicle.Speed = 0.0, door closed, currently locked
    // 2. Send lock command with action "unlock"
    // 3. Wait for response
    // 4. Verify IsLocked becomes false
    panic!("not implemented: requires running LOCKING_SERVICE and databroker-client");
}

/// TS-02-10: LOCKING_SERVICE rejects lock when vehicle moving (02-REQ-3.1)
///
/// Verify LOCKING_SERVICE rejects a lock command when Vehicle.Speed > 0.
#[test]
#[ignore = "requires DATA_BROKER + LOCKING_SERVICE running"]
fn test_locking_rejects_lock_vehicle_moving() {
    require_infra!();

    // Steps:
    // 1. Record initial IsLocked state
    // 2. Set Vehicle.Speed = 30.0
    // 3. Send lock command with action "lock"
    // 4. Wait for response
    // 5. Verify response status = "failed", reason = "vehicle_moving"
    // 6. Verify IsLocked is NOT changed
    panic!("not implemented: requires running LOCKING_SERVICE and databroker-client");
}

/// TS-02-11: LOCKING_SERVICE rejects lock when door open (02-REQ-3.2)
///
/// Verify LOCKING_SERVICE rejects a lock command when the door is open.
#[test]
#[ignore = "requires DATA_BROKER + LOCKING_SERVICE running"]
fn test_locking_rejects_lock_door_open() {
    require_infra!();

    // Steps:
    // 1. Record initial IsLocked state
    // 2. Set Vehicle.Speed = 0.0, IsOpen = true
    // 3. Send lock command with action "lock"
    // 4. Wait for response
    // 5. Verify response status = "failed", reason = "door_open"
    // 6. Verify IsLocked is NOT changed
    panic!("not implemented: requires running LOCKING_SERVICE and databroker-client");
}

/// TS-02-12: LOCKING_SERVICE rejects unlock when vehicle moving (02-REQ-3.3)
///
/// Verify LOCKING_SERVICE rejects an unlock command when Vehicle.Speed > 0.
#[test]
#[ignore = "requires DATA_BROKER + LOCKING_SERVICE running"]
fn test_locking_rejects_unlock_vehicle_moving() {
    require_infra!();

    // Steps:
    // 1. Set locked = true, speed = 15.0
    // 2. Send lock command with action "unlock"
    // 3. Wait for response
    // 4. Verify response status = "failed", reason = "vehicle_moving"
    // 5. Verify IsLocked remains true
    panic!("not implemented: requires running LOCKING_SERVICE and databroker-client");
}

/// TS-02-13: LOCKING_SERVICE writes failure response with reason (02-REQ-3.4)
///
/// Verify the failure response includes the specific constraint violation reason.
#[test]
#[ignore = "requires DATA_BROKER + LOCKING_SERVICE running"]
fn test_locking_failure_response_has_reason() {
    require_infra!();

    // Steps:
    // 1. Set speed = 10.0 (violates safety constraint)
    // 2. Send lock command
    // 3. Wait for response
    // 4. Verify response JSON contains status "failed" and a non-empty reason
    // 5. Verify command_id is present in response
    panic!("not implemented: requires running LOCKING_SERVICE and databroker-client");
}

/// TS-02-14: LOCKING_SERVICE writes success response and lock state (02-REQ-3.5)
///
/// Verify successful lock command writes both lock state and success response.
#[test]
#[ignore = "requires DATA_BROKER + LOCKING_SERVICE running"]
fn test_locking_success_response_and_state() {
    require_infra!();

    // Steps:
    // 1. Set speed = 0.0, door closed
    // 2. Generate unique command_id
    // 3. Send lock command with action "lock"
    // 4. Wait for response
    // 5. Verify response status = "success" and command_id matches
    // 6. Verify IsLocked = true
    panic!("not implemented: requires running LOCKING_SERVICE and databroker-client");
}
