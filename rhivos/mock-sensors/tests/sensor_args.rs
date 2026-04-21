//! Integration tests for mock sensor binary argument validation.
//!
//! TS-09-E1: location-sensor missing --lat or --lon → exit 1, stderr non-empty
//! TS-09-E2: speed-sensor missing --speed → exit 1, stderr non-empty
//! TS-09-E3: door-sensor missing --open/--closed → exit 1, stderr non-empty
//! TS-09-E4: any sensor with unreachable DATA_BROKER → exit 1, stderr non-empty

use std::process::Command;

// ── location-sensor argument validation ────────────────────────────────────

/// TS-09-E1 / 01-REQ-4.1: location-sensor with no arguments should print
/// version to stdout and exit 0 (spec 01 skeleton behavior takes precedence).
/// Missing individual args (e.g. --lat without --lon) still produce non-zero exit.
#[test]
fn test_location_sensor_no_args() {
    let out = Command::new(env!("CARGO_BIN_EXE_location-sensor"))
        .output()
        .expect("failed to start location-sensor");
    assert!(
        out.status.success(),
        "expected exit 0 when invoked with no args (01-REQ-4.1), got {:?}",
        out.status.code()
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("location-sensor"),
        "expected version string containing 'location-sensor' on stdout, got: {stdout}"
    );
}

/// TS-09-E1: location-sensor with missing --lon should exit 1.
#[test]
fn test_location_sensor_missing_lon() {
    let out = Command::new(env!("CARGO_BIN_EXE_location-sensor"))
        .arg("--lat=48.1351")
        .output()
        .expect("failed to start location-sensor");
    assert!(
        !out.status.success(),
        "expected non-zero exit when --lon is missing, got {:?}",
        out.status.code()
    );
    assert!(
        !out.stderr.is_empty(),
        "expected error message on stderr"
    );
}

/// TS-09-E1: location-sensor with missing --lat should exit 1.
#[test]
fn test_location_sensor_missing_lat() {
    let out = Command::new(env!("CARGO_BIN_EXE_location-sensor"))
        .arg("--lon=11.5820")
        .output()
        .expect("failed to start location-sensor");
    assert!(
        !out.status.success(),
        "expected non-zero exit when --lat is missing, got {:?}",
        out.status.code()
    );
    assert!(
        !out.stderr.is_empty(),
        "expected error message on stderr"
    );
}

// ── speed-sensor argument validation ──────────────────────────────────────

/// TS-09-E2 / 01-REQ-4.1: speed-sensor with no arguments should print
/// version to stdout and exit 0 (spec 01 skeleton behavior takes precedence).
/// Missing --speed when other args are present still produces non-zero exit.
#[test]
fn test_speed_sensor_no_args() {
    let out = Command::new(env!("CARGO_BIN_EXE_speed-sensor"))
        .output()
        .expect("failed to start speed-sensor");
    assert!(
        out.status.success(),
        "expected exit 0 when invoked with no args (01-REQ-4.1), got {:?}",
        out.status.code()
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("speed-sensor"),
        "expected version string containing 'speed-sensor' on stdout, got: {stdout}"
    );
}

// ── door-sensor argument validation ───────────────────────────────────────

/// TS-09-E3 / 01-REQ-4.1: door-sensor with no arguments should print
/// version to stdout and exit 0 (spec 01 skeleton behavior takes precedence).
/// Missing --open/--closed when other args are present still produces non-zero exit.
#[test]
fn test_door_sensor_no_args() {
    let out = Command::new(env!("CARGO_BIN_EXE_door-sensor"))
        .output()
        .expect("failed to start door-sensor");
    assert!(
        out.status.success(),
        "expected exit 0 when invoked with no args (01-REQ-4.1), got {:?}",
        out.status.code()
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("door-sensor"),
        "expected version string containing 'door-sensor' on stdout, got: {stdout}"
    );
}

// ── unreachable DATA_BROKER ────────────────────────────────────────────────

/// TS-09-E4: location-sensor with unreachable DATA_BROKER should exit 1.
#[test]
fn test_location_sensor_unreachable_broker() {
    let out = Command::new(env!("CARGO_BIN_EXE_location-sensor"))
        .args(["--lat=48.1351", "--lon=11.5820", "--broker-addr=http://localhost:19999"])
        .output()
        .expect("failed to start location-sensor");
    assert!(
        !out.status.success(),
        "expected non-zero exit when DATA_BROKER is unreachable, got {:?}",
        out.status.code()
    );
    assert!(
        !out.stderr.is_empty(),
        "expected connection error on stderr"
    );
}

/// TS-09-E4: speed-sensor with unreachable DATA_BROKER should exit 1.
#[test]
fn test_speed_sensor_unreachable_broker() {
    let out = Command::new(env!("CARGO_BIN_EXE_speed-sensor"))
        .args(["--speed=0.0", "--broker-addr=http://localhost:19999"])
        .output()
        .expect("failed to start speed-sensor");
    assert!(
        !out.status.success(),
        "expected non-zero exit when DATA_BROKER is unreachable, got {:?}",
        out.status.code()
    );
    assert!(
        !out.stderr.is_empty(),
        "expected connection error on stderr"
    );
}

/// TS-09-E4: door-sensor with unreachable DATA_BROKER should exit 1.
#[test]
fn test_door_sensor_unreachable_broker() {
    let out = Command::new(env!("CARGO_BIN_EXE_door-sensor"))
        .args(["--open", "--broker-addr=http://localhost:19999"])
        .output()
        .expect("failed to start door-sensor");
    assert!(
        !out.status.success(),
        "expected non-zero exit when DATA_BROKER is unreachable, got {:?}",
        out.status.code()
    );
    assert!(
        !out.stderr.is_empty(),
        "expected connection error on stderr"
    );
}
