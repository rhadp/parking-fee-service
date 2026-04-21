//! Integration tests for mock sensor binary argument validation.
//!
//! TS-09-E1: location-sensor missing --lat or --lon → exit 1, stderr non-empty
//! TS-09-E2: speed-sensor missing --speed → exit 1, stderr non-empty
//! TS-09-E3: door-sensor missing --open/--closed → exit 1, stderr non-empty
//! TS-09-E4: any sensor with unreachable DATA_BROKER → exit 1, stderr non-empty

use std::process::Command;

// ── location-sensor argument validation ────────────────────────────────────

/// TS-09-E1: location-sensor with no arguments should exit non-zero with
/// a usage error on stderr (09-REQ-1.E1 — missing --lat and --lon).
#[test]
fn test_location_sensor_no_args() {
    let out = Command::new(env!("CARGO_BIN_EXE_location-sensor"))
        .output()
        .expect("failed to start location-sensor");
    assert!(
        !out.status.success(),
        "expected non-zero exit when invoked with no args (09-REQ-1.E1), got {:?}",
        out.status.code()
    );
    assert!(
        !out.stderr.is_empty(),
        "expected usage error on stderr"
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

/// TS-09-E2: speed-sensor with no arguments should exit non-zero with
/// a usage error on stderr (09-REQ-2.E1 — missing --speed).
#[test]
fn test_speed_sensor_no_args() {
    let out = Command::new(env!("CARGO_BIN_EXE_speed-sensor"))
        .output()
        .expect("failed to start speed-sensor");
    assert!(
        !out.status.success(),
        "expected non-zero exit when invoked with no args (09-REQ-2.E1), got {:?}",
        out.status.code()
    );
    assert!(
        !out.stderr.is_empty(),
        "expected usage error on stderr"
    );
}

// ── door-sensor argument validation ───────────────────────────────────────

/// TS-09-E3: door-sensor with no arguments should exit non-zero with
/// a usage error on stderr (09-REQ-3.E1 — missing --open/--closed).
#[test]
fn test_door_sensor_no_args() {
    let out = Command::new(env!("CARGO_BIN_EXE_door-sensor"))
        .output()
        .expect("failed to start door-sensor");
    assert!(
        !out.status.success(),
        "expected non-zero exit when invoked with no args (09-REQ-3.E1), got {:?}",
        out.status.code()
    );
    assert!(
        !out.stderr.is_empty(),
        "expected usage error on stderr"
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

// ── TS-09-P2: CLI argument validation property test ──────────────────────────
//
// Property 2 from design.md: For any invocation with missing required arguments,
// the tool exits with a non-zero code and prints an error to stderr.
// Tests multiple argument subset combinations per binary (09-REQ-1.E1, 09-REQ-2.E1, 09-REQ-3.E1).

/// TS-09-P2: location-sensor — all incomplete argument subsets exit non-zero.
/// Invariant: any invocation missing at least one of --lat or --lon must fail.
#[test]
fn test_location_sensor_arg_validation_property() {
    // All subsets of required args where at least one is missing.
    let invalid_arg_sets: &[&[&str]] = &[
        &[],                             // no args at all
        &["--lat=48.13"],                 // missing --lon
        &["--lon=11.58"],                 // missing --lat
        &["--broker-addr=http://x:1"],    // only optional flag, missing required
        &["--lat=48.13", "--broker-addr=http://x:1"], // missing --lon with optional
        &["--lon=11.58", "--broker-addr=http://x:1"], // missing --lat with optional
    ];

    for (i, args) in invalid_arg_sets.iter().enumerate() {
        let out = Command::new(env!("CARGO_BIN_EXE_location-sensor"))
            .args(*args)
            .output()
            .unwrap_or_else(|e| panic!("case {i}: failed to start: {e}"));
        assert!(
            !out.status.success(),
            "case {i}: expected non-zero exit for args {:?}, got {:?}",
            args,
            out.status.code()
        );
        assert!(
            !out.stderr.is_empty(),
            "case {i}: expected error on stderr for args {:?}",
            args
        );
    }
}

/// TS-09-P2: speed-sensor — all incomplete argument subsets exit non-zero.
/// Invariant: any invocation missing --speed must fail.
#[test]
fn test_speed_sensor_arg_validation_property() {
    let invalid_arg_sets: &[&[&str]] = &[
        &[],                             // no args at all
        &["--broker-addr=http://x:1"],   // only optional flag
    ];

    for (i, args) in invalid_arg_sets.iter().enumerate() {
        let out = Command::new(env!("CARGO_BIN_EXE_speed-sensor"))
            .args(*args)
            .output()
            .unwrap_or_else(|e| panic!("case {i}: failed to start: {e}"));
        assert!(
            !out.status.success(),
            "case {i}: expected non-zero exit for args {:?}, got {:?}",
            args,
            out.status.code()
        );
        assert!(
            !out.stderr.is_empty(),
            "case {i}: expected error on stderr for args {:?}",
            args
        );
    }
}

/// TS-09-P2: door-sensor — all incomplete argument subsets exit non-zero.
/// Invariant: any invocation missing both --open and --closed must fail.
#[test]
fn test_door_sensor_arg_validation_property() {
    let invalid_arg_sets: &[&[&str]] = &[
        &[],                             // no args at all
        &["--broker-addr=http://x:1"],   // only optional flag
    ];

    for (i, args) in invalid_arg_sets.iter().enumerate() {
        let out = Command::new(env!("CARGO_BIN_EXE_door-sensor"))
            .args(*args)
            .output()
            .unwrap_or_else(|e| panic!("case {i}: failed to start: {e}"));
        assert!(
            !out.status.success(),
            "case {i}: expected non-zero exit for args {:?}, got {:?}",
            args,
            out.status.code()
        );
        assert!(
            !out.stderr.is_empty(),
            "case {i}: expected error on stderr for args {:?}",
            args
        );
    }
}

/// TS-09-P2: door-sensor — --open and --closed are mutually exclusive.
/// Providing both should result in a usage error.
#[test]
fn test_door_sensor_mutual_exclusion() {
    let out = Command::new(env!("CARGO_BIN_EXE_door-sensor"))
        .args(["--open", "--closed"])
        .output()
        .expect("failed to start door-sensor");
    assert!(
        !out.status.success(),
        "expected non-zero exit when both --open and --closed are given, got {:?}",
        out.status.code()
    );
    assert!(
        !out.stderr.is_empty(),
        "expected error on stderr for mutually exclusive flags"
    );
}
