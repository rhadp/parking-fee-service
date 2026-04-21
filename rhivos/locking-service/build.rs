fn main() {
    tonic_build::configure()
        .build_server(false)
        .compile_protos(
            &["proto/kuksa/val/v1/val.proto"],
            &["proto"],
        )
        .expect("failed to compile kuksa.val.v1 proto files");
}
