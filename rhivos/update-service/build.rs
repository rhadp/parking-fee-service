fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().compile(
        &["proto/update/update_service.proto"],
        &["proto"],
    )?;
    Ok(())
}
