//! Generated proto types for gRPC communication.
//!
//! - `parking`: parking.adaptor service (ParkingAdaptor gRPC server)
//! - `kuksa`: kuksa.val.v1 service (Kuksa Databroker gRPC client)

/// Generated types for the `parking.adaptor` gRPC service.
pub mod parking {
    tonic::include_proto!("parking.adaptor");
}

/// Generated types for the `kuksa.val.v1` Databroker gRPC service.
pub mod kuksa {
    tonic::include_proto!("kuksa.val.v1");
}
