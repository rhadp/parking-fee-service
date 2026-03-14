//! Generated protobuf types for the UPDATE_SERVICE gRPC API.
//!
//! The code is generated at build time by `tonic-build` from
//! `proto/update_service.proto` and `proto/common.proto`.

/// Common parking types shared across services.
pub mod common {
    include!(concat!(env!("OUT_DIR"), "/parking.common.rs"));
}

/// UpdateService-specific request/response types and generated server stub.
pub mod updateservice {
    include!(concat!(env!("OUT_DIR"), "/parking.updateservice.rs"));
}
