//! Mock sensor integration tests
//!
//! These tests verify mock sensor CLI tools (LOCATION_SENSOR, SPEED_SENSOR,
//! DOOR_SENSOR) write correct values to DATA_BROKER and handle errors properly.
//!
//! Test Spec: TS-02-21, TS-02-22, TS-02-23, TS-02-24, TS-02-25

use std::process::Command;

/// Helper: check if DATA_BROKER infrastructure is available.
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
            eprintln!("SKIP: DATA_BROKER not available on port 55556 (run `make infra-up`)");
            return;
        }
    };
}

/// TS-02-21: LOCATION_SENSOR CLI writes latitude and longitude (02-REQ-6.1)
///
/// Verify LOCATION_SENSOR CLI tool writes location signals to DATA_BROKER.
#[test]
#[ignore = "requires DATA_BROKER infrastructure and built sensor binaries"]
fn test_location_sensor_writes() {
    require_infra!();

    // Steps:
    // 1. Run: location-sensor --lat 48.1351 --lon 11.5820
    // 2. Verify exit code = 0
    // 3. Read Vehicle.CurrentLocation.Latitude from DATA_BROKER
    // 4. Read Vehicle.CurrentLocation.Longitude from DATA_BROKER
    // 5. Verify values match within tolerance (0.0001)
    panic!("not implemented: requires built location-sensor binary and databroker-client");
}

/// TS-02-22: SPEED_SENSOR CLI writes speed (02-REQ-6.2)
///
/// Verify SPEED_SENSOR CLI tool writes speed signal to DATA_BROKER.
#[test]
#[ignore = "requires DATA_BROKER infrastructure and built sensor binaries"]
fn test_speed_sensor_writes() {
    require_infra!();

    // Steps:
    // 1. Run: speed-sensor --speed 55.5
    // 2. Verify exit code = 0
    // 3. Read Vehicle.Speed from DATA_BROKER
    // 4. Verify value is approximately 55.5 (within 0.1)
    panic!("not implemented: requires built speed-sensor binary and databroker-client");
}

/// TS-02-23: DOOR_SENSOR CLI writes door state (02-REQ-6.3)
///
/// Verify DOOR_SENSOR CLI tool writes door open/closed state to DATA_BROKER.
#[test]
#[ignore = "requires DATA_BROKER infrastructure and built sensor binaries"]
fn test_door_sensor_writes() {
    require_infra!();

    // Steps:
    // 1. Run: door-sensor --open true
    // 2. Verify exit code = 0
    // 3. Read Vehicle.Cabin.Door.Row1.DriverSide.IsOpen from DATA_BROKER
    // 4. Verify value = true
    // 5. Run: door-sensor --open false
    // 6. Verify exit code = 0
    // 7. Verify value = false
    panic!("not implemented: requires built door-sensor binary and databroker-client");
}

/// TS-02-24: Mock sensor tools exit 0 on success (02-REQ-6.4)
///
/// Verify each mock sensor tool exits with code 0 after successfully writing
/// a value.
#[test]
#[ignore = "requires DATA_BROKER infrastructure and built sensor binaries"]
fn test_sensor_exit_code_success() {
    require_infra!();

    // Steps:
    // 1. Run speed-sensor --speed 0 -> verify exit code 0
    // 2. Run door-sensor --open false -> verify exit code 0
    // 3. Run location-sensor --lat 0 --lon 0 -> verify exit code 0
    panic!("not implemented: requires built sensor binaries");
}

/// TS-02-25: Mock sensor tools show usage without arguments (02-REQ-6.5)
///
/// Verify each mock sensor tool displays usage when run without required
/// arguments. This test does NOT require infrastructure.
#[test]
fn test_sensor_usage_without_args() {
    let sensors = ["speed-sensor", "door-sensor", "location-sensor"];

    for sensor in &sensors {
        // Try to find the binary in the target directory
        let binary = format!("target/debug/{}", sensor);
        let binary_path = std::path::Path::new(&binary);

        // If binary doesn't exist, try building first
        if !binary_path.exists() {
            // Try cargo build for the binary
            let build_result = Command::new("cargo")
                .args(["build", "--bin", sensor])
                .output();

            match build_result {
                Ok(output) if !output.status.success() => {
                    panic!(
                        "could not build {}: binary not found and cargo build failed: {}",
                        sensor,
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                Err(e) => {
                    panic!("could not build {}: {}", sensor, e);
                }
                _ => {}
            }
        }

        let result = Command::new(&binary).output();

        match result {
            Ok(output) => {
                assert_ne!(
                    output.status.code().unwrap_or(-1),
                    0,
                    "{} should exit with non-zero code when run without arguments",
                    sensor
                );
                let combined =
                    String::from_utf8_lossy(&output.stdout).to_string()
                        + &String::from_utf8_lossy(&output.stderr);
                assert!(
                    combined.contains("Usage")
                        || combined.contains("usage")
                        || combined.contains("--"),
                    "{} output should contain usage information, got: {}",
                    sensor,
                    combined
                );
            }
            Err(_) => {
                panic!(
                    "{} binary not found at {} — mock-sensors crate not built",
                    sensor, binary
                );
            }
        }
    }
}
