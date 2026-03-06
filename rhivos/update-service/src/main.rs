pub mod config;
pub mod container;
pub mod grpc;
pub mod manager;
pub mod oci;
pub mod offload;
pub mod state;

/// Generated protobuf types for update_service.v1.
#[allow(clippy::doc_overindented_list_items)]
pub mod proto {
    tonic::include_proto!("update_service.v1");
}

fn main() {
    println!("update-service starting...");
}
