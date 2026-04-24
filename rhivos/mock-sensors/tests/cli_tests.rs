//! Integration tests for mock sensor CLI argument validation and error handling.
//!
//! These tests invoke sensor binaries via std::process::Command and verify
//! exit codes and stderr output. They cover:
//! - TS-09-E1: location-sensor missing args
//! - TS-09-E2: speed-sensor missing args
//! - TS-09-E3: door-sensor missing args
//! - TS-09-E4: sensor unreachable DATA_BROKER
//! - TS-09-E12: door-sensor mutually exclusive flags

use std::process::Command;

/// Helper to get the path to a built binary within the workspace.
fn binary_path(name: &str) -> std::path::PathBuf {
    let mut path = std::env::current_exe()
        .expect("failed to get current exe path");
    // Navigate from test binary to the target debug directory
    path.pop(); // remove test binary name
    path.pop(); // remove deps/
    path.push(name);
    path
}

// ---------------------------------------------------------------------------
// TS-09-E1: Location Sensor Missing Args
// Requirement: 09-REQ-1.E1
// ---------------------------------------------------------------------------

#[test]
fn test_location_sensor_missing_lon() {
    // TS-09-E1: location-sensor with missing --lon exits 1 with usage error.
    let output = Command::new(binary_path("location-sensor"))
        .args(["--lat=48.13"])
        .output()
        .expect("failed to execute location-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "location-sensor with missing --lon should exit 1, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stderr.is_empty(),
        "location-sensor should print error to stderr when --lon is missing"
    );
}

#[test]
fn test_location_sensor_missing_lat() {
    // TS-09-E1: location-sensor with missing --lat exits 1 with usage error.
    let output = Command::new(binary_path("location-sensor"))
        .args(["--lon=11.58"])
        .output()
        .expect("failed to execute location-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "location-sensor with missing --lat should exit 1"
    );
    assert!(
        !output.stderr.is_empty(),
        "location-sensor should print error to stderr when --lat is missing"
    );
}

#[test]
fn test_location_sensor_no_args() {
    // TS-09-E1: location-sensor with no args exits 1.
    let output = Command::new(binary_path("location-sensor"))
        .output()
        .expect("failed to execute location-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "location-sensor with no args should exit 1"
    );
    assert!(
        !output.stderr.is_empty(),
        "location-sensor should print error to stderr when args are missing"
    );
}

// ---------------------------------------------------------------------------
// TS-09-E2: Speed Sensor Missing Args
// Requirement: 09-REQ-2.E1
// ---------------------------------------------------------------------------

#[test]
fn test_speed_sensor_missing_speed() {
    // TS-09-E2: speed-sensor with missing --speed exits 1.
    let output = Command::new(binary_path("speed-sensor"))
        .output()
        .expect("failed to execute speed-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "speed-sensor with no args should exit 1"
    );
    assert!(
        !output.stderr.is_empty(),
        "speed-sensor should print error to stderr when --speed is missing"
    );
}

// ---------------------------------------------------------------------------
// TS-09-E3: Door Sensor Missing Args
// Requirement: 09-REQ-3.E1
// ---------------------------------------------------------------------------

#[test]
fn test_door_sensor_missing_args() {
    // TS-09-E3: door-sensor with neither --open nor --closed exits 1.
    let output = Command::new(binary_path("door-sensor"))
        .output()
        .expect("failed to execute door-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "door-sensor with no args should exit 1"
    );
    assert!(
        !output.stderr.is_empty(),
        "door-sensor should print error to stderr when --open/--closed is missing"
    );
}

// ---------------------------------------------------------------------------
// TS-09-E12: Door Sensor Mutually Exclusive Flags
// Requirement: 09-REQ-3.E3
// ---------------------------------------------------------------------------

#[test]
fn test_door_sensor_mutual_exclusion() {
    // TS-09-E12: door-sensor with both --open and --closed exits 1.
    let output = Command::new(binary_path("door-sensor"))
        .args(["--open", "--closed"])
        .output()
        .expect("failed to execute door-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "door-sensor with both --open and --closed should exit 1, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.stderr.is_empty(),
        "door-sensor should print usage error to stderr when both --open and --closed are given"
    );
}

// ---------------------------------------------------------------------------
// TS-09-E4: Sensor Unreachable DATA_BROKER
// Requirement: 09-REQ-1.E2, 09-REQ-2.E2, 09-REQ-3.E2
// ---------------------------------------------------------------------------

#[test]
fn test_location_sensor_unreachable_broker() {
    // TS-09-E4: location-sensor exits 1 when DATA_BROKER is unreachable.
    let output = Command::new(binary_path("location-sensor"))
        .args([
            "--lat=48.13",
            "--lon=11.58",
            "--broker-addr=http://localhost:19999",
        ])
        .env_remove("DATABROKER_ADDR")
        .output()
        .expect("failed to execute location-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "location-sensor should exit 1 when broker is unreachable"
    );
    assert!(
        !output.stderr.is_empty(),
        "location-sensor should print connection error to stderr"
    );
}

#[test]
fn test_speed_sensor_unreachable_broker() {
    // TS-09-E4: speed-sensor exits 1 when DATA_BROKER is unreachable.
    let output = Command::new(binary_path("speed-sensor"))
        .args([
            "--speed=60.0",
            "--broker-addr=http://localhost:19999",
        ])
        .env_remove("DATABROKER_ADDR")
        .output()
        .expect("failed to execute speed-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "speed-sensor should exit 1 when broker is unreachable"
    );
    assert!(
        !output.stderr.is_empty(),
        "speed-sensor should print connection error to stderr"
    );
}

#[test]
fn test_door_sensor_unreachable_broker() {
    // TS-09-E4: door-sensor exits 1 when DATA_BROKER is unreachable.
    let output = Command::new(binary_path("door-sensor"))
        .args([
            "--open",
            "--broker-addr=http://localhost:19999",
        ])
        .env_remove("DATABROKER_ADDR")
        .output()
        .expect("failed to execute door-sensor");

    assert_eq!(
        output.status.code(),
        Some(1),
        "door-sensor should exit 1 when broker is unreachable"
    );
    assert!(
        !output.stderr.is_empty(),
        "door-sensor should print connection error to stderr"
    );
}
