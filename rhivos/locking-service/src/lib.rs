//! LOCKING_SERVICE library crate.
//!
//! Exposes the core modules for use in integration tests and the binary.

pub mod command;
pub mod config;
pub mod databroker_client;
pub mod executor;
pub mod safety;

/// Generated Kuksa VAL v2 protobuf types.
#[allow(clippy::doc_overindented_list_items)]
pub mod kuksa_proto {
    tonic::include_proto!("kuksa.val.v2");
}
