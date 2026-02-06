//! PARKING_OPERATOR_ADAPTOR library
//!
//! This library provides components for parking session management
//! in the RHIVOS QM partition.
//!
//! # Overview
//!
//! The PARKING_OPERATOR_ADAPTOR:
//! - Subscribes to lock/unlock events from DATA_BROKER
//! - Automatically starts/stops parking sessions based on lock state
//! - Communicates with external PARKING_OPERATOR via REST API
//! - Provides session status to PARKING_APP via gRPC

pub mod config;
pub mod error;
pub mod location;
pub mod logging;
pub mod manager;
pub mod operator;
pub mod poller;
pub mod publisher;
pub mod service;
pub mod session;
pub mod store;
pub mod subscriber;
pub mod zone;

#[cfg(test)]
pub mod test_utils;

// Proto-generated code
#[allow(clippy::all)]
pub mod proto {
    pub mod parking {
        include!("proto/sdv.services.parking.rs");
    }
}

// Re-exports for convenience
pub use config::ServiceConfig;
pub use error::{ApiError, ParkingError};
pub use location::{Location, LocationReader};
pub use logging::{init_tracing, EventType, LogEntry, Logger};
pub use manager::SessionManager;
pub use operator::{OperatorApiClient, StartRequest, StartResponse, StopRequest, StopResponse, StatusResponse};
pub use poller::StatusPoller;
pub use publisher::StatePublisher;
pub use service::ParkingAdaptorImpl;
pub use session::{Session, SessionState};
pub use store::SessionStore;
pub use subscriber::SignalSubscriber;
pub use zone::{ZoneInfo, ZoneLookupClient};
