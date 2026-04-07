fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/parking_adaptor.proto")?;
    tonic_build::compile_protos("../../proto/kuksa/val.proto")?;
    Ok(())
}
