//! UPDATE_SERVICE - Container Lifecycle Management Library
//!
//! This crate provides the core functionality for the RHIVOS update service,
//! managing containerized parking operator adapters.
//!
//! # Architecture
//!
//! The service consists of the following components:
//! - **UpdateServiceImpl**: Main gRPC service implementation
//! - **RegistryAuthenticator**: Handles OCI registry authentication
//! - **ImageDownloader**: Downloads container images with retry logic
//! - **AttestationValidator**: Validates container attestations
//! - **ContainerManager**: Manages container lifecycle via podman
//! - **StateTracker**: Tracks adapter states
//! - **WatcherManager**: Manages streaming subscriptions
//! - **OffloadScheduler**: Schedules automatic offloading
//! - **OperationLogger**: Provides structured logging
//!
//! # Communication
//!
//! - Receives commands from PARKING_APP via gRPC/TLS
//! - Downloads adapters from REGISTRY via HTTPS/OCI
//! - Manages containers via podman

pub mod attestation;
pub mod authenticator;
pub mod config;
pub mod container;
pub mod downloader;
pub mod error;
pub mod logger;
pub mod offload;
pub mod service;
pub mod state;
pub mod tracker;
pub mod watcher;

#[cfg(test)]
pub mod test_utils;

/// Re-export proto types from shared crate for convenience
pub mod proto {
    pub use shared::sdv::services::update::*;
}

// Re-export commonly used types
pub use attestation::AttestationValidator;
pub use authenticator::RegistryAuthenticator;
pub use config::ServiceConfig;
pub use container::ContainerManager;
pub use downloader::ImageDownloader;
pub use error::{UpdateError, UpdateResult};
pub use logger::OperationLogger;
pub use offload::OffloadScheduler;
pub use service::UpdateServiceImpl;
pub use state::AdapterEntry;
pub use tracker::StateTracker;
pub use watcher::WatcherManager;
