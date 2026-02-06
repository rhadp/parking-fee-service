//! Build script for cloud-gateway-client
//!
//! Generates gRPC client code from Protocol Buffer definitions.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure tonic-build for LOCKING_SERVICE client
    tonic_build::configure()
        .build_server(false) // We only need the client
        .build_client(true)
        .out_dir("src/proto")
        .compile_protos(
            &["../../proto/services/locking_service.proto"],
            &["../../proto"],
        )?;

    // Tell Cargo to rerun if the proto file changes
    println!("cargo:rerun-if-changed=../../proto/services/locking_service.proto");

    Ok(())
}
