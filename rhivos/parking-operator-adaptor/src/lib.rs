//! PARKING_OPERATOR_ADAPTOR library.
//!
//! This crate implements the ParkingAdaptor gRPC service that bridges
//! the PARKING_APP with the PARKING_OPERATOR REST API. It manages parking
//! sessions both autonomously (via lock/unlock events from DATA_BROKER)
//! and manually (via gRPC calls from PARKING_APP).

pub mod config;
pub mod databroker_client;
pub mod event_handler;
pub mod grpc_service;
pub mod operator_client;
pub mod session_manager;

/// Generated proto types for the ParkingAdaptor service.
pub mod proto {
    pub mod adaptor {
        tonic::include_proto!("parking.adaptor.v1");
    }
}
