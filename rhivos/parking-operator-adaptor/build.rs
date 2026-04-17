fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile kuksa.val proto — client only (we consume DATA_BROKER, not serve it).
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile(&["proto/kuksa/val.proto"], &["proto"])?;

    // Compile ParkingAdaptor proto — server only (we serve this interface).
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(&["proto/parking_adaptor.proto"], &["proto"])?;

    Ok(())
}
