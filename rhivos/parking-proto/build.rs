use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Proto source directory relative to the workspace root
    let proto_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("proto");

    let proto_files = [
        proto_dir.join("common/common.proto"),
        proto_dir.join("services/update_service.proto"),
        proto_dir.join("services/parking_adapter.proto"),
    ];

    // Include path is the proto root so imports like "common/common.proto" resolve
    let include_dirs = [&proto_dir];

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&proto_files, &include_dirs)?;

    // Re-run build if any proto file changes
    for proto_file in &proto_files {
        println!("cargo:rerun-if-changed={}", proto_file.display());
    }

    Ok(())
}
