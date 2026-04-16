/// Integration tests for mock sensor binaries.
///
/// These tests are in the RED phase: stubs always exit 0, so every test that
/// expects exit code 1 (missing args / unreachable broker) will FAIL until
/// task group 2 implements the real argument parsing and gRPC logic.
use std::path::PathBuf;
use std::process::Command;

/// Returns the path to a compiled mock-sensor binary.
///
/// Cargo builds binaries to `<workspace>/target/debug/<name>`.
/// CARGO_MANIFEST_DIR = rhivos/mock-sensors, so the workspace root is one
/// directory up.
fn sensor_binary(name: &str) -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest.parent().expect("manifest has parent");
    workspace_root.join("target").join("debug").join(name)
}

// ────────────────────────────────────────────────────────────
// TS-09-E1: location-sensor — missing required arguments
// ────────────────────────────────────────────────────────────

/// TS-09-E1: location-sensor with missing --lon exits 1 and writes to stderr.
#[test]
fn test_location_sensor_missing_lon() {
    let out = Command::new(sensor_binary("location-sensor"))
        .arg("--lat=48.1351")
        .output()
        .expect("failed to spawn location-sensor");
    assert!(
        !out.status.success(),
        "expected exit code 1 for missing --lon, got 0"
    );
    assert!(
        !out.stderr.is_empty(),
        "expected error message on stderr for missing --lon"
    );
}

/// TS-09-E1: location-sensor with missing --lat exits 1 and writes to stderr.
#[test]
fn test_location_sensor_missing_lat() {
    let out = Command::new(sensor_binary("location-sensor"))
        .arg("--lon=11.5820")
        .output()
        .expect("failed to spawn location-sensor");
    assert!(
        !out.status.success(),
        "expected exit code 1 for missing --lat, got 0"
    );
    assert!(
        !out.stderr.is_empty(),
        "expected error message on stderr for missing --lat"
    );
}

/// TS-09-E1: location-sensor with no arguments exits 1 and writes to stderr.
#[test]
fn test_location_sensor_no_args() {
    let out = Command::new(sensor_binary("location-sensor"))
        .output()
        .expect("failed to spawn location-sensor");
    assert!(
        !out.status.success(),
        "expected exit code 1 for no args, got 0"
    );
    assert!(
        !out.stderr.is_empty(),
        "expected error message on stderr for no args"
    );
}

// ────────────────────────────────────────────────────────────
// TS-09-E2: speed-sensor — missing required argument
// ────────────────────────────────────────────────────────────

/// TS-09-E2: speed-sensor with no arguments exits 1 and writes to stderr.
#[test]
fn test_speed_sensor_no_args() {
    let out = Command::new(sensor_binary("speed-sensor"))
        .output()
        .expect("failed to spawn speed-sensor");
    assert!(
        !out.status.success(),
        "expected exit code 1 for missing --speed, got 0"
    );
    assert!(
        !out.stderr.is_empty(),
        "expected error message on stderr for missing --speed"
    );
}

// ────────────────────────────────────────────────────────────
// TS-09-E3: door-sensor — missing required argument
// ────────────────────────────────────────────────────────────

/// TS-09-E3: door-sensor with neither --open nor --closed exits 1.
#[test]
fn test_door_sensor_no_state_flag() {
    let out = Command::new(sensor_binary("door-sensor"))
        .output()
        .expect("failed to spawn door-sensor");
    assert!(
        !out.status.success(),
        "expected exit code 1 for missing --open/--closed, got 0"
    );
    assert!(
        !out.stderr.is_empty(),
        "expected error message on stderr"
    );
}

// ────────────────────────────────────────────────────────────
// TS-09-E4: all sensors — DATA_BROKER unreachable
// ────────────────────────────────────────────────────────────

/// TS-09-E4: location-sensor with unreachable broker exits 1.
#[test]
fn test_location_sensor_unreachable_broker() {
    let out = Command::new(sensor_binary("location-sensor"))
        .args([
            "--lat=48.1351",
            "--lon=11.5820",
            "--broker-addr=http://localhost:19999",
        ])
        .output()
        .expect("failed to spawn location-sensor");
    assert!(
        !out.status.success(),
        "expected exit code 1 for unreachable broker, got 0"
    );
    assert!(
        !out.stderr.is_empty(),
        "expected connection error on stderr"
    );
}

/// TS-09-E4: speed-sensor with unreachable broker exits 1.
#[test]
fn test_speed_sensor_unreachable_broker() {
    let out = Command::new(sensor_binary("speed-sensor"))
        .args(["--speed=0.0", "--broker-addr=http://localhost:19999"])
        .output()
        .expect("failed to spawn speed-sensor");
    assert!(
        !out.status.success(),
        "expected exit code 1 for unreachable broker, got 0"
    );
    assert!(
        !out.stderr.is_empty(),
        "expected connection error on stderr"
    );
}

/// TS-09-E4: door-sensor with unreachable broker exits 1.
#[test]
fn test_door_sensor_unreachable_broker() {
    let out = Command::new(sensor_binary("door-sensor"))
        .args(["--open", "--broker-addr=http://localhost:19999"])
        .output()
        .expect("failed to spawn door-sensor");
    assert!(
        !out.status.success(),
        "expected exit code 1 for unreachable broker, got 0"
    );
    assert!(
        !out.stderr.is_empty(),
        "expected connection error on stderr"
    );
}
