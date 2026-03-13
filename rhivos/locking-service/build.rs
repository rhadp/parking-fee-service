fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .compile(
            &["proto/kuksa/val/v1/val.proto", "proto/kuksa/val/v1/types.proto"],
            &["proto"],
        )?;
    Ok(())
}
