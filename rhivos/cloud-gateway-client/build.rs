fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .compile(
            &["proto/kuksa/val/v2/val.proto"],
            &["proto"],
        )?;
    Ok(())
}
