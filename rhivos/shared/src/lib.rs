//! Shared library for RHIVOS services
//!
//! This crate provides common types, utilities, and configurations
//! used across all RHIVOS Rust services.

pub mod config;
pub mod error;

/// Re-export commonly used types
pub use config::{DataBrokerConfig, MqttConfig, ServiceConfig};
pub use error::{Error, Result};
