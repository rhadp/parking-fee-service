fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile parking_adaptor.proto (gRPC server + client)
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &["../../proto/parking_adaptor.proto"],
            &["../../proto"],
        )?;

    // Compile Kuksa Databroker proto (client only, for DATA_BROKER interaction)
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(
            &["../../proto/kuksa/val/v2/val.proto"],
            &["../../proto"],
        )?;

    Ok(())
}
