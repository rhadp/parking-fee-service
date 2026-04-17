fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(&["proto/parking_adaptor.proto"], &["proto"])?;

    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile(&["proto/kuksa/val.proto"], &["proto"])?;

    Ok(())
}
