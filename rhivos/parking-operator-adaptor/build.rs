fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile the parking_adaptor gRPC service definition
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &["../../proto/parking_adaptor.proto"],
            &["../../proto"],
        )?;

    // Compile the Kuksa Databroker proto for DATA_BROKER integration
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(
            &["../../tests/setup/proto/kuksa/val/v2/val.proto"],
            &["../../tests/setup/proto"],
        )?;

    Ok(())
}
