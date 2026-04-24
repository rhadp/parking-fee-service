fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().compile(
        &[
            "proto/kuksa/val.proto",
            "proto/adapter/adapter_service.proto",
        ],
        &["proto"],
    )?;
    Ok(())
}
