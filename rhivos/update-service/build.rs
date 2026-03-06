fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &["../../proto/update_service/v1/update_service.proto"],
            &["../../proto"],
        )?;
    Ok(())
}
