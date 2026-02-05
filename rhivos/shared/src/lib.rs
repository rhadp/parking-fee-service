//! Shared library for RHIVOS services
//!
//! This crate provides common types, utilities, and configurations
//! used across all RHIVOS Rust services.

pub mod config;
pub mod error;
pub mod proto;

/// Re-export commonly used types
pub use config::{DataBrokerConfig, MqttConfig, ServiceConfig};
pub use error::{Error, Result};

/// Re-export proto types for convenience
pub use proto::sdv;
