fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Proto files live at the repo root under proto/.
    // CARGO_MANIFEST_DIR is rhivos/update-service/, so we go up two levels.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let proto_root = std::path::Path::new(&manifest_dir)
        .parent() // rhivos/
        .unwrap()
        .parent() // repo root
        .unwrap()
        .join("proto");

    let proto_file = proto_root.join("update/update_service.proto");

    tonic_build::configure().compile(
        &[proto_file.to_str().unwrap()],
        &[proto_root.to_str().unwrap()],
    )?;
    Ok(())
}
