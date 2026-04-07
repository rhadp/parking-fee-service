pub mod adapter;
pub mod config;
pub mod grpc;
pub mod monitor;
pub mod offload;
pub mod podman;
pub mod service;
pub mod state;

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod proptest_cases;

pub mod proto {
    tonic::include_proto!("update");
}
