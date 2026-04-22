//! CLOUD_GATEWAY_CLIENT — bridges DATA_BROKER with CLOUD_GATEWAY via NATS.
//!
//! This service runs in the RHIVOS safety partition and implements three
//! data flows:
//! 1. Inbound command processing (NATS -> DATA_BROKER)
//! 2. Outbound command response relay (DATA_BROKER -> NATS)
//! 3. Outbound telemetry publishing (DATA_BROKER -> NATS)

pub mod broker_client;
pub mod command_validator;
pub mod config;
pub mod errors;
pub mod models;
pub mod nats_client;
pub mod telemetry;

#[cfg(test)]
pub mod proptest_cases;

fn main() {
    println!("cloud-gateway-client v0.1.0");
}
