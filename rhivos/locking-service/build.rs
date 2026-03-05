fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .compile_protos(
            &["../../tests/setup/proto/kuksa/val/v2/val.proto"],
            &["../../tests/setup/proto"],
        )?;
    Ok(())
}
