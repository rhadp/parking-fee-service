fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().compile_protos(
        &[
            "../../proto/update_service.proto",
            "../../proto/common.proto",
        ],
        &["../../proto/"],
    )?;
    Ok(())
}
