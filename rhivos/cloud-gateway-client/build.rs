fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/kuksa/val/v1/val.proto")?;
    Ok(())
}
