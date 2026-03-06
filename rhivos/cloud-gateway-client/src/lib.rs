//! CLOUD_GATEWAY_CLIENT library.
//!
//! Bridges DATA_BROKER and CLOUD_GATEWAY via NATS messaging.
//! This library crate exposes the modules for use by integration tests
//! and the binary entry point in `main.rs`.

pub mod command;
pub mod command_processor;
pub mod config;
pub mod databroker_client;
pub mod nats_client;
pub mod response_relay;
pub mod telemetry;

/// Generated Kuksa VAL v2 protobuf types.
#[allow(clippy::doc_overindented_list_items)]
pub mod kuksa_proto {
    tonic::include_proto!("kuksa.val.v2");
}
