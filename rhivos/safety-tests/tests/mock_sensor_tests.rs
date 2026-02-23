//! Mock sensor integration tests
//!
//! These tests verify mock sensor CLI tools (LOCATION_SENSOR, SPEED_SENSOR,
//! DOOR_SENSOR) write correct values to DATA_BROKER and handle errors properly.
//!
//! Test Spec: TS-02-21, TS-02-22, TS-02-23, TS-02-24, TS-02-25

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use databroker_client::DatabrokerClient;

/// Helper: check if DATA_BROKER infrastructure is available.
fn infra_available() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:55556".parse().unwrap(),
        Duration::from_secs(2),
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

/// Connect to DATA_BROKER via TCP for testing.
async fn test_client() -> DatabrokerClient {
    DatabrokerClient::connect("http://localhost:55556")
        .await
        .expect("should connect to DATA_BROKER on port 55556")
}

/// Helper: find the workspace target directory and return the sensor binary path.
///
/// Cargo places binaries in the workspace-level `target/debug/` directory.
/// When running tests, `CARGO_MANIFEST_DIR` points to the crate root
/// (`safety-tests/`), so we go up one level to find the workspace root.
///
/// If the binary doesn't exist, builds it with `cargo build --bin {name} -p mock-sensors`.
fn sensor_binary(name: &str) -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR should be set by cargo");
    let workspace_root = PathBuf::from(&manifest_dir)
        .parent()
        .expect("safety-tests should be inside workspace")
        .to_path_buf();
    let binary = workspace_root.join("target").join("debug").join(name);

    if !binary.exists() {
        let build_result = Command::new("cargo")
            .args(["build", "--bin", name, "-p", "mock-sensors"])
            .current_dir(&workspace_root)
            .output();

        match build_result {
            Ok(output) if !output.status.success() => {
                panic!(
                    "could not build {}: cargo build failed: {}",
                    name,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(e) => {
                panic!("could not build {}: {}", name, e);
            }
            _ => {}
        }
    }

    assert!(
        binary.exists(),
        "{} binary not found at {} — mock-sensors crate not built",
        name,
        binary.display()
    );

    binary
}

/// TS-02-21: LOCATION_SENSOR CLI writes latitude and longitude (02-REQ-6.1)
///
/// Verify LOCATION_SENSOR CLI tool writes location signals to DATA_BROKER.
#[tokio::test]
async fn test_location_sensor_writes() {
    require_infra!();

    let binary = sensor_binary("location-sensor");
    let client = test_client().await;

    let output = Command::new(&binary)
        .args(["--lat", "48.1351", "--lon", "11.5820"])
        .env("DATABROKER_ADDR", "http://localhost:55556")
        .output()
        .expect("should run location-sensor");

    assert!(
        output.status.success(),
        "location-sensor should exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let lat = client
        .get_value("Vehicle.CurrentLocation.Latitude")
        .await
        .expect("should read latitude");
    let lon = client
        .get_value("Vehicle.CurrentLocation.Longitude")
        .await
        .expect("should read longitude");

    let lat_val = lat.as_double().expect("latitude should be a double");
    let lon_val = lon.as_double().expect("longitude should be a double");

    assert!(
        (lat_val - 48.1351).abs() < 0.001,
        "latitude should be ~48.1351, got {}",
        lat_val
    );
    assert!(
        (lon_val - 11.582).abs() < 0.001,
        "longitude should be ~11.582, got {}",
        lon_val
    );
}

/// TS-02-22: SPEED_SENSOR CLI writes speed (02-REQ-6.2)
///
/// Verify SPEED_SENSOR CLI tool writes speed signal to DATA_BROKER.
#[tokio::test]
async fn test_speed_sensor_writes() {
    require_infra!();

    let binary = sensor_binary("speed-sensor");
    let client = test_client().await;

    let output = Command::new(&binary)
        .args(["--speed", "55.5"])
        .env("DATABROKER_ADDR", "http://localhost:55556")
        .output()
        .expect("should run speed-sensor");

    assert!(
        output.status.success(),
        "speed-sensor should exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let speed = client
        .get_value("Vehicle.Speed")
        .await
        .expect("should read Vehicle.Speed");

    let speed_val = speed.as_float().expect("speed should be a float");

    assert!(
        (speed_val - 55.5).abs() < 0.5,
        "speed should be ~55.5, got {}",
        speed_val
    );
}

/// TS-02-23: DOOR_SENSOR CLI writes door state (02-REQ-6.3)
///
/// Verify DOOR_SENSOR CLI tool writes door open/closed state to DATA_BROKER.
#[tokio::test]
async fn test_door_sensor_writes() {
    require_infra!();

    let binary = sensor_binary("door-sensor");
    let client = test_client().await;

    // Set door open
    let output = Command::new(&binary)
        .args(["--open", "true"])
        .env("DATABROKER_ADDR", "http://localhost:55556")
        .output()
        .expect("should run door-sensor");

    assert!(
        output.status.success(),
        "door-sensor --open true should exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let is_open = client
        .get_value("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
        .await
        .expect("should read IsOpen");

    assert_eq!(
        is_open.as_bool(),
        Some(true),
        "door should be open after --open true"
    );

    // Set door closed
    let output = Command::new(&binary)
        .args(["--open", "false"])
        .env("DATABROKER_ADDR", "http://localhost:55556")
        .output()
        .expect("should run door-sensor");

    assert!(
        output.status.success(),
        "door-sensor --open false should exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let is_open = client
        .get_value("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
        .await
        .expect("should read IsOpen");

    assert_eq!(
        is_open.as_bool(),
        Some(false),
        "door should be closed after --open false"
    );
}

/// TS-02-24: Mock sensor tools exit 0 on success (02-REQ-6.4)
///
/// Verify each mock sensor tool exits with code 0 after successfully writing
/// a value.
#[tokio::test]
async fn test_sensor_exit_code_success() {
    require_infra!();

    let sensors = [
        ("speed-sensor", vec!["--speed", "0"]),
        ("door-sensor", vec!["--open", "false"]),
        ("location-sensor", vec!["--lat", "0", "--lon", "0"]),
    ];

    for (name, args) in &sensors {
        let binary = sensor_binary(name);
        let output = Command::new(&binary)
            .args(args)
            .env("DATABROKER_ADDR", "http://localhost:55556")
            .output()
            .unwrap_or_else(|e| panic!("{} binary could not be executed: {}", name, e));

        assert!(
            output.status.success(),
            "{} should exit 0 on success, got {:?}, stderr: {}",
            name,
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// TS-02-25: Mock sensor tools show usage without arguments (02-REQ-6.5)
///
/// Verify each mock sensor tool displays usage when run without required
/// arguments. This test does NOT require infrastructure.
#[test]
fn test_sensor_usage_without_args() {
    let sensors = ["speed-sensor", "door-sensor", "location-sensor"];

    for sensor in &sensors {
        let binary = sensor_binary(sensor);

        let output = Command::new(&binary)
            .output()
            .unwrap_or_else(|e| {
                panic!(
                    "{} binary could not be executed at {}: {}",
                    sensor,
                    binary.display(),
                    e
                )
            });

        assert_ne!(
            output.status.code().unwrap_or(-1),
            0,
            "{} should exit with non-zero code when run without arguments",
            sensor
        );
        let combined = String::from_utf8_lossy(&output.stdout).to_string()
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
}
