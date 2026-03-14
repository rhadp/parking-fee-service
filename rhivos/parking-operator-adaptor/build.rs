//! Build script for parking-operator-adaptor.
//!
//! Compiles two proto packages:
//!   1. `proto/parking_adaptor.proto` → generates the server-side ParkingAdaptor trait.
//!   2. `proto/kuksa/val/v1/*.proto`  → generates the DATA_BROKER gRPC client types.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parking adaptor API: server-side generation (we serve this RPC).
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(&["proto/parking_adaptor.proto"], &["proto"])?;

    // Kuksa.val.v1 DATA_BROKER API: client-side generation only (we subscribe/set).
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile(
            &[
                "proto/kuksa/val/v1/val.proto",
                "proto/kuksa/val/v1/types.proto",
            ],
            &["proto"],
        )?;

    Ok(())
}
