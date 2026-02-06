fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = std::path::Path::new("../../proto");

    // Compile parking_adaptor.proto
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/proto")
        .compile(
            &[proto_root.join("services/parking_adaptor.proto")],
            &[proto_root],
        )?;

    // Re-run if proto file changes
    println!(
        "cargo:rerun-if-changed={}",
        proto_root.join("services/parking_adaptor.proto").display()
    );

    Ok(())
}
