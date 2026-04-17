// Integration tests for mock sensor binaries.
// These tests invoke the compiled binaries via std::process::Command.
// CARGO_BIN_EXE_* macros are only available in integration tests.
//
// In task group 1 (RED phase), all binary invocation tests FAIL because
// the stub binaries exit 0 regardless of arguments.
// Task group 2 will implement real argument validation and broker communication.

use std::process::Command;

// TS-09-E1: location-sensor with missing --lon exits 1 with usage error.
#[test]
fn test_location_sensor_missing_lon() {
    let output = Command::new(env!("CARGO_BIN_EXE_location-sensor"))
        .arg("--lat=48.13")
        .output()
        .expect("failed to spawn location-sensor");
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 when --lon is missing; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stderr.is_empty(),
        "expected non-empty stderr when --lon is missing"
    );
}

// TS-09-E1: location-sensor with missing --lat exits 1.
#[test]
fn test_location_sensor_missing_lat() {
    let output = Command::new(env!("CARGO_BIN_EXE_location-sensor"))
        .arg("--lon=11.58")
        .output()
        .expect("failed to spawn location-sensor");
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 when --lat is missing"
    );
    assert!(
        !output.stderr.is_empty(),
        "expected non-empty stderr when --lat is missing"
    );
}

// TS-09-E1: location-sensor with no args exits 1.
#[test]
fn test_location_sensor_no_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_location-sensor"))
        .output()
        .expect("failed to spawn location-sensor");
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 with no args"
    );
}

// TS-09-E2: speed-sensor with missing --speed exits 1.
#[test]
fn test_speed_sensor_missing_speed() {
    let output = Command::new(env!("CARGO_BIN_EXE_speed-sensor"))
        .output()
        .expect("failed to spawn speed-sensor");
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 when --speed is missing"
    );
    assert!(
        !output.stderr.is_empty(),
        "expected non-empty stderr when --speed is missing"
    );
}

// TS-09-E2: speed-sensor with no args exits 1 (TS-09-P2 property).
#[test]
fn test_speed_sensor_no_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_speed-sensor"))
        .output()
        .expect("failed to spawn speed-sensor");
    assert_eq!(
        output.status.code(),
        Some(1),
        "speed-sensor with no args must exit 1"
    );
}

// TS-09-E3: door-sensor with neither --open nor --closed exits 1.
#[test]
fn test_door_sensor_missing_state() {
    let output = Command::new(env!("CARGO_BIN_EXE_door-sensor"))
        .output()
        .expect("failed to spawn door-sensor");
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 when neither --open nor --closed is provided"
    );
    assert!(
        !output.stderr.is_empty(),
        "expected non-empty stderr when door state flag is missing"
    );
}

// TS-09-E3: door-sensor with no args exits 1 (TS-09-P2 property).
#[test]
fn test_door_sensor_no_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_door-sensor"))
        .output()
        .expect("failed to spawn door-sensor");
    assert_eq!(
        output.status.code(),
        Some(1),
        "door-sensor with no args must exit 1"
    );
}

// TS-09-E4: location-sensor with unreachable DATA_BROKER exits 1.
#[test]
fn test_location_sensor_unreachable_broker() {
    let output = Command::new(env!("CARGO_BIN_EXE_location-sensor"))
        .args(["--lat=48.13", "--lon=11.58", "--broker-addr=http://localhost:19999"])
        .output()
        .expect("failed to spawn location-sensor");
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 when DATA_BROKER is unreachable"
    );
    assert!(
        !output.stderr.is_empty(),
        "expected connection error on stderr when broker is unreachable"
    );
}

// TS-09-E4: speed-sensor with unreachable DATA_BROKER exits 1.
#[test]
fn test_speed_sensor_unreachable_broker() {
    let output = Command::new(env!("CARGO_BIN_EXE_speed-sensor"))
        .args(["--speed=0.0", "--broker-addr=http://localhost:19999"])
        .output()
        .expect("failed to spawn speed-sensor");
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 when DATA_BROKER is unreachable"
    );
    assert!(
        !output.stderr.is_empty(),
        "expected connection error on stderr"
    );
}

// TS-09-E4: door-sensor with unreachable DATA_BROKER exits 1.
#[test]
fn test_door_sensor_unreachable_broker() {
    let output = Command::new(env!("CARGO_BIN_EXE_door-sensor"))
        .args(["--open", "--broker-addr=http://localhost:19999"])
        .output()
        .expect("failed to spawn door-sensor");
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 when DATA_BROKER is unreachable"
    );
    assert!(
        !output.stderr.is_empty(),
        "expected connection error on stderr"
    );
}
