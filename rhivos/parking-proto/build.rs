use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Proto source directory relative to the workspace root
    let proto_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("proto");

    // ── Parking-domain protos ───────────────────────────────────────────
    let parking_protos = [
        proto_dir.join("common/common.proto"),
        proto_dir.join("services/update_service.proto"),
        proto_dir.join("services/parking_adapter.proto"),
    ];

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&parking_protos, &[&proto_dir])?;

    for proto_file in &parking_protos {
        println!("cargo:rerun-if-changed={}", proto_file.display());
    }

    // ── Kuksa val.v2 protos (vendored) ──────────────────────────────────
    let vendor_dir = proto_dir.join("vendor");

    let kuksa_protos = [
        vendor_dir.join("kuksa/val/v2/types.proto"),
        vendor_dir.join("kuksa/val/v2/val.proto"),
    ];

    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&kuksa_protos, &[&vendor_dir])?;

    for proto_file in &kuksa_protos {
        println!("cargo:rerun-if-changed={}", proto_file.display());
    }

    Ok(())
}
