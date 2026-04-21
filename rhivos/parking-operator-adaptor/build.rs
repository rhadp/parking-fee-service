fn main() {
    // Compile kuksa.val.v1 proto — client only (no server needed).
    tonic_build::configure()
        .build_server(false)
        .compile(
            &["proto/kuksa/val/v1/val.proto"],
            &["proto"],
        )
        .expect("failed to compile kuksa.val.v1 proto files");

    // Compile parking_adaptor proto — server only (client built by integration tests).
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(
            &["proto/parking_adaptor.proto"],
            &["proto"],
        )
        .expect("failed to compile parking_adaptor proto");
}
