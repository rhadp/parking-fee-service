//! UPDATE_SERVICE library.
//!
//! This crate implements the UpdateService gRPC service that manages
//! the lifecycle of containerized adapters, including installation,
//! state tracking, OCI image pulling, checksum verification, and
//! automatic offloading. The adapter lifecycle follows a well-defined
//! state machine (04-REQ-7.1).

pub mod adapter_manager;
pub mod checksum;
pub mod config;
pub mod container_runtime;
pub mod grpc_service;
pub mod oci_client;
pub mod offloader;

/// Generated proto types for the UpdateService.
///
/// Module hierarchy mirrors the proto package structure so that
/// cross-package references (e.g. `super::super::common::v1::AdapterState`)
/// resolve correctly.
pub mod parking {
    pub mod common {
        pub mod v1 {
            tonic::include_proto!("parking.common.v1");
        }
    }
    pub mod update {
        pub mod v1 {
            tonic::include_proto!("parking.update.v1");
        }
    }
}
