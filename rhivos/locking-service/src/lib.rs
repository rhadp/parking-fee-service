//! LOCKING_SERVICE - ASIL-B Door Locking Service Library
//!
//! This crate provides the core functionality for the RHIVOS door locking service,
//! including safety constraint validation, lock execution, and state publication.
//!
//! # Architecture
//!
//! The service consists of the following components:
//! - **LockingServiceImpl**: Main gRPC service implementation
//! - **SafetyValidator**: Validates safety constraints before command execution
//! - **LockExecutor**: Executes lock/unlock operations
//! - **StatePublisher**: Publishes state changes to DATA_BROKER
//!
//! # Communication
//!
//! - Receives commands from CLOUD_GATEWAY_CLIENT via gRPC/UDS
//! - Publishes door state to DATA_BROKER via gRPC/UDS

pub mod config;
pub mod error;
pub mod state;

/// Re-export proto types from shared crate for convenience
pub mod proto {
    pub use shared::sdv::services::locking::*;
}

// Re-export commonly used types
pub use config::ServiceConfig;
pub use error::{LockingError, Result, SafetyViolation};
pub use state::{DoorState, LockState};
