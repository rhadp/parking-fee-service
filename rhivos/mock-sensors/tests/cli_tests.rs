//! Integration tests for mock sensor CLI argument validation and error handling.
//!
//! These tests invoke the compiled sensor binaries via `std::process::Command`
//! and verify exit codes and stderr output. They cover:
//!
//! - TS-09-E1: location-sensor missing --lat or --lon
//! - TS-09-E2: speed-sensor missing --speed
//! - TS-09-E3: door-sensor missing --open/--closed
//! - TS-09-E4: sensors with unreachable DATA_BROKER
//! - TS-09-E12: door-sensor with both --open and --closed

use std::process::Command;

// ---------------------------------------------------------------------------
// TS-09-E1: Location Sensor Missing Args
// Requirement: 09-REQ-1.E1
// ---------------------------------------------------------------------------

#[test]
fn test_location_sensor_missing_lon() {
    let output = Command::new(env!("CARGO_BIN_EXE_location-sensor"))
        .arg("--lat=48.13")
        .output()
        .expect("failed to execute location-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1 when --lon is missing, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stderr.is_empty(),
        "expected usage error on stderr when --lon is missing"
    );
}

#[test]
fn test_location_sensor_missing_lat() {
    let output = Command::new(env!("CARGO_BIN_EXE_location-sensor"))
        .arg("--lon=11.58")
        .output()
        .expect("failed to execute location-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1 when --lat is missing, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stderr.is_empty(),
        "expected usage error on stderr when --lat is missing"
    );
}

#[test]
fn test_location_sensor_no_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_location-sensor"))
        .output()
        .expect("failed to execute location-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1 when no args provided, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stderr.is_empty(),
        "expected usage error on stderr when no args provided"
    );
}

// ---------------------------------------------------------------------------
// TS-09-E2: Speed Sensor Missing Args
// Requirement: 09-REQ-2.E1
// ---------------------------------------------------------------------------

#[test]
fn test_speed_sensor_missing_speed() {
    let output = Command::new(env!("CARGO_BIN_EXE_speed-sensor"))
        .output()
        .expect("failed to execute speed-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1 when --speed is missing, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stderr.is_empty(),
        "expected usage error on stderr when --speed is missing"
    );
}

// ---------------------------------------------------------------------------
// TS-09-E3: Door Sensor Missing Args
// Requirement: 09-REQ-3.E1
// ---------------------------------------------------------------------------

#[test]
fn test_door_sensor_missing_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_door-sensor"))
        .output()
        .expect("failed to execute door-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1 when neither --open nor --closed is provided, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stderr.is_empty(),
        "expected usage error on stderr when no door state flag provided"
    );
}

// ---------------------------------------------------------------------------
// TS-09-E12: Door Sensor Mutually Exclusive Flags
// Requirement: 09-REQ-3.E3
// ---------------------------------------------------------------------------

#[test]
fn test_door_sensor_mutual_exclusion() {
    let output = Command::new(env!("CARGO_BIN_EXE_door-sensor"))
        .args(["--open", "--closed"])
        .output()
        .expect("failed to execute door-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1 when both --open and --closed are provided, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stderr.is_empty(),
        "expected usage error on stderr when both --open and --closed provided"
    );
}

// ---------------------------------------------------------------------------
// TS-09-E4: Sensor Unreachable DATA_BROKER
// Requirements: 09-REQ-1.E2, 09-REQ-2.E2, 09-REQ-3.E2
// ---------------------------------------------------------------------------

#[test]
fn test_location_sensor_unreachable_broker() {
    let output = Command::new(env!("CARGO_BIN_EXE_location-sensor"))
        .args([
            "--lat=48.13",
            "--lon=11.58",
            "--broker-addr=http://localhost:19999",
        ])
        .output()
        .expect("failed to execute location-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1 when DATA_BROKER is unreachable, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stderr.is_empty(),
        "expected connection error on stderr when DATA_BROKER is unreachable"
    );
}

#[test]
fn test_speed_sensor_unreachable_broker() {
    let output = Command::new(env!("CARGO_BIN_EXE_speed-sensor"))
        .args(["--speed=50.0", "--broker-addr=http://localhost:19999"])
        .output()
        .expect("failed to execute speed-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1 when DATA_BROKER is unreachable, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stderr.is_empty(),
        "expected connection error on stderr when DATA_BROKER is unreachable"
    );
}

#[test]
fn test_door_sensor_unreachable_broker() {
    let output = Command::new(env!("CARGO_BIN_EXE_door-sensor"))
        .args(["--open", "--broker-addr=http://localhost:19999"])
        .output()
        .expect("failed to execute door-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1 when DATA_BROKER is unreachable, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stderr.is_empty(),
        "expected connection error on stderr when DATA_BROKER is unreachable"
    );
}
