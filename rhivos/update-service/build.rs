fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../../proto/update/update_service.proto")?;
    Ok(())
}
