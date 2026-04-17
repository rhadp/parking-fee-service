pub mod adapter;
pub mod config;
pub mod grpc_handler;
pub mod monitor;
pub mod offload;
pub mod podman;
pub mod service;
pub mod state;

pub mod proto {
    tonic::include_proto!("update");
}
