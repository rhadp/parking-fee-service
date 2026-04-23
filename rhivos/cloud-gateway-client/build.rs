fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .compile_protos(
            &[
                "proto/kuksa/val/v1/val.proto",
                "proto/kuksa/val/v2/val.proto",
            ],
            &["proto"],
        )?;
    Ok(())
}
