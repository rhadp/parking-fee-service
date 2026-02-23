//! CLOUD_GATEWAY_CLIENT — MQTT bridge between cloud gateway and vehicle DATA_BROKER.
//!
//! This service maintains an MQTT connection to the CLOUD_GATEWAY broker
//! and bridges commands and telemetry between the cloud and the vehicle's
//! DATA_BROKER (Eclipse Kuksa Databroker).
//!
//! Communication with DATA_BROKER is exclusively via gRPC over Unix Domain
//! Sockets (UDS) for same-partition isolation.
//!
//! ## Responsibilities
//!
//! - Connect to MQTT broker and subscribe to command topics
//! - Validate incoming command messages from MQTT
//! - Write validated commands to DATA_BROKER as command signals
//! - Subscribe to DATA_BROKER for vehicle state and command responses
//! - Publish telemetry and command responses to MQTT

pub mod commands;
pub mod mqtt;
pub mod service;
pub mod telemetry;
