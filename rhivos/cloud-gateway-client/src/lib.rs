//! CLOUD_GATEWAY_CLIENT library — shared modules for the cloud-gateway-client
//! service.
//!
//! Exposes the configuration, validation, telemetry, NATS client, and
//! DATA_BROKER client modules so that both the binary and integration tests
//! can access them.

pub mod broker_client;
pub mod command_validator;
pub mod config;
pub mod errors;
pub mod models;
pub mod nats_client;
pub mod telemetry;

#[cfg(test)]
pub mod proptest_cases;
