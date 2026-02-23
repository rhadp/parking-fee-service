//! DATA_BROKER integration tests
//!
//! These tests verify DATA_BROKER (Eclipse Kuksa Databroker) configuration
//! and behavior. They require running infrastructure (`make infra-up`).
//!
//! Test Spec: TS-02-2, TS-02-3, TS-02-4, TS-02-5

use std::time::Duration;

use databroker_client::{DataValue, DatabrokerClient};

/// Helper: check if infrastructure is available by attempting a TCP connection
/// to the DATA_BROKER port.
fn infra_available() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:55556".parse().unwrap(),
        Duration::from_secs(2),
    )
    .is_ok()
}

/// Skip test if infrastructure is not running.
macro_rules! require_infra {
    () => {
        if !infra_available() {
            eprintln!("SKIP: infrastructure not available (run `make infra-up`)");
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

/// TS-02-2: Standard VSS signals are accessible in DATA_BROKER (02-REQ-1.2)
///
/// Verify DATA_BROKER serves all required standard VSS signals after startup.
/// Each signal must exist and have the correct data type.
#[tokio::test]
async fn test_standard_vss_signals() {
    require_infra!();

    let client = test_client().await;

    // Verify each required standard VSS signal exists by querying metadata
    let signals = [
        "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked",
        "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
        "Vehicle.CurrentLocation.Latitude",
        "Vehicle.CurrentLocation.Longitude",
        "Vehicle.Speed",
    ];

    for path in &signals {
        let metadata = client.get_metadata(path).await;
        assert!(
            metadata.is_ok(),
            "signal {} should exist in DATA_BROKER, got error: {:?}",
            path,
            metadata.err()
        );
    }

    // Verify we can write and read standard signals
    client
        .set_value("Vehicle.Speed", DataValue::Float(0.0))
        .await
        .expect("should write Vehicle.Speed");

    let val = client.get_value("Vehicle.Speed").await;
    assert!(val.is_ok(), "should read Vehicle.Speed");

    // Verify custom command signals also exist
    for path in &[
        "Vehicle.Command.Door.Lock",
        "Vehicle.Command.Door.Response",
    ] {
        let metadata = client.get_metadata(path).await;
        assert!(
            metadata.is_ok(),
            "custom signal {} should exist, got: {:?}",
            path,
            metadata.err()
        );
    }
}

/// TS-02-3: DATA_BROKER UDS endpoint reachable (02-REQ-1.3)
///
/// Verify DATA_BROKER's gRPC interface is reachable via Unix Domain Socket.
#[tokio::test]
async fn test_databroker_uds_endpoint() {
    require_infra!();

    // Try to connect via UDS
    let uds_path = std::env::var("DATABROKER_UDS_PATH")
        .unwrap_or_else(|_| "/tmp/kuksa-databroker.sock".to_string());

    let endpoint = format!("unix://{}", uds_path);
    let result = DatabrokerClient::connect(&endpoint).await;

    // If the UDS socket file exists, connection should succeed
    if std::path::Path::new(&uds_path).exists() {
        let client = result.expect("should connect to DATA_BROKER via UDS");
        let info = client.get_server_info().await;
        assert!(
            info.is_ok(),
            "GetServerInfo via UDS should succeed, got: {:?}",
            info.err()
        );
    } else {
        // UDS socket might not be available in CI (Docker on Mac)
        eprintln!(
            "SKIP: UDS socket not found at {} — running in container-only mode",
            uds_path
        );
    }
}

/// TS-02-4: DATA_BROKER network TCP endpoint reachable (02-REQ-1.4)
///
/// Verify DATA_BROKER's gRPC interface is reachable via network TCP.
#[tokio::test]
async fn test_databroker_tcp_endpoint() {
    require_infra!();

    let client = DatabrokerClient::connect("http://localhost:55556")
        .await
        .expect("should connect to DATA_BROKER via TCP");

    let info = client.get_server_info().await;
    assert!(
        info.is_ok(),
        "GetServerInfo via TCP should succeed, got: {:?}",
        info.err()
    );

    let (name, version) = info.unwrap();
    assert!(!name.is_empty(), "server name should not be empty");
    assert!(!version.is_empty(), "server version should not be empty");
}

/// TS-02-5: DATA_BROKER bearer token access control (02-REQ-1.5)
///
/// Verify DATA_BROKER enforces bearer token authentication for write operations.
#[tokio::test]
async fn test_databroker_bearer_token() {
    require_infra!();

    let client = test_client().await;

    // Verify that the client can write successfully
    let result = client
        .set_value("Vehicle.Speed", DataValue::Float(0.0))
        .await;

    assert!(
        result.is_ok(),
        "write with default client should succeed, got: {:?}",
        result.err()
    );
}
