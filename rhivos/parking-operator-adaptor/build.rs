fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/kuksa/val/v1/val.proto")?;
    tonic_build::compile_protos("proto/parking_adaptor/v1/adapter_service.proto")?;
    Ok(())
}
