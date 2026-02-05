// Build script for generating Rust Protocol Buffer bindings
// This file is used by cargo to generate gRPC/protobuf code at build time
//
// Requirements: 2.7, 4.9

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = std::path::Path::new("../../proto");
    let out_dir = std::path::Path::new("src/proto");
    
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(out_dir)?;
    
    // Collect all proto files
    let proto_files: Vec<_> = walkdir::WalkDir::new(proto_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "proto"))
        .map(|e| e.path().to_path_buf())
        .collect();
    
    if proto_files.is_empty() {
        println!("cargo:warning=No proto files found in {}", proto_root.display());
        return Ok(());
    }
    
    // Configure tonic-build
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(out_dir)
        .compile(&proto_files, &[proto_root])?;
    
    // Re-run if any proto file changes
    for proto in &proto_files {
        println!("cargo:rerun-if-changed={}", proto.display());
    }
    
    // Re-run if build.rs changes
    println!("cargo:rerun-if-changed=build.rs");
    
    Ok(())
}
