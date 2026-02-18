use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("proto");

    let proto_files = [
        proto_dir.join("kuksa/val/v1/types.proto"),
        proto_dir.join("kuksa/val/v1/val.proto"),
    ];

    let include_dirs = [&proto_dir];

    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&proto_files, &include_dirs)?;

    for proto_file in &proto_files {
        println!("cargo:rerun-if-changed={}", proto_file.display());
    }

    Ok(())
}
