//! DATA_BROKER integration tests
//!
//! These tests verify DATA_BROKER (Eclipse Kuksa Databroker) configuration
//! and behavior. They require running infrastructure (`make infra-up`).
//!
//! Test Spec: TS-02-2, TS-02-3, TS-02-4, TS-02-5

/// Helper: check if infrastructure is available by attempting a TCP connection
/// to the DATA_BROKER port.
fn infra_available() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:55556".parse().unwrap(),
        std::time::Duration::from_secs(2),
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

/// TS-02-2: Standard VSS signals are accessible in DATA_BROKER (02-REQ-1.2)
///
/// Verify DATA_BROKER serves all required standard VSS signals after startup.
/// Each signal must exist and have the correct data type.
#[test]
#[ignore = "requires DATA_BROKER infrastructure and databroker-client crate"]
fn test_standard_vss_signals() {
    require_infra!();

    // Signals to verify with expected types:
    // - Vehicle.Cabin.Door.Row1.DriverSide.IsLocked: bool
    // - Vehicle.Cabin.Door.Row1.DriverSide.IsOpen: bool
    // - Vehicle.CurrentLocation.Latitude: double
    // - Vehicle.CurrentLocation.Longitude: double
    // - Vehicle.Speed: float

    // TODO: Use databroker-client to query signal metadata
    panic!("not implemented: requires databroker-client crate to query VSS signal metadata");
}

/// TS-02-3: DATA_BROKER UDS endpoint reachable (02-REQ-1.3)
///
/// Verify DATA_BROKER's gRPC interface is reachable via Unix Domain Socket.
#[test]
#[ignore = "requires DATA_BROKER infrastructure and databroker-client crate"]
fn test_databroker_uds_endpoint() {
    require_infra!();

    // TODO: Connect via UDS at /tmp/kuksa-databroker.sock
    // A simple GetServerInfo call should return without error.
    panic!("not implemented: requires databroker-client crate with UDS support");
}

/// TS-02-4: DATA_BROKER network TCP endpoint reachable (02-REQ-1.4)
///
/// Verify DATA_BROKER's gRPC interface is reachable via network TCP.
#[test]
#[ignore = "requires DATA_BROKER infrastructure and databroker-client crate"]
fn test_databroker_tcp_endpoint() {
    require_infra!();

    // TODO: Connect via TCP at http://localhost:55556
    // A simple GetServerInfo call should return without error.
    panic!("not implemented: requires databroker-client crate with TCP support");
}

/// TS-02-5: DATA_BROKER bearer token access control (02-REQ-1.5)
///
/// Verify DATA_BROKER enforces bearer token authentication for write operations.
#[test]
#[ignore = "requires DATA_BROKER infrastructure with token configuration"]
fn test_databroker_bearer_token() {
    require_infra!();

    // TODO: Write with valid token should succeed.
    // Write without token should be rejected with permission denied.
    panic!("not implemented: requires databroker-client crate and token configuration");
}
