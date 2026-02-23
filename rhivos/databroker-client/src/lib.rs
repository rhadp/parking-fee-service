//! Typed Rust client for Eclipse Kuksa Databroker gRPC API (`kuksa.val.v1`).
//!
//! This crate provides a high-level client for interacting with the Kuksa
//! Databroker's gRPC API. It supports connection over both Unix Domain
//! Sockets (UDS) and TCP endpoints.
//!
//! # Usage
//!
//! ```no_run
//! use databroker_client::{DatabrokerClient, DataValue};
//!
//! # async fn example() -> Result<(), databroker_client::Error> {
//! // Connect via UDS (same-partition)
//! let client = DatabrokerClient::connect("unix:///tmp/kuksa-databroker.sock").await?;
//!
//! // Read a signal
//! let speed = client.get_value("Vehicle.Speed").await?;
//!
//! // Write a signal
//! client.set_value("Vehicle.Speed", DataValue::Float(42.0)).await?;
//! # Ok(())
//! # }
//! ```

/// Re-export of the generated Kuksa gRPC types.
pub mod proto {
    pub mod kuksa {
        pub mod val {
            pub mod v1 {
                tonic::include_proto!("kuksa.val.v1");
            }
        }
    }
}

mod client;
mod error;
mod value;

pub use client::{DatabrokerClient, SignalStream, SignalUpdate, DEFAULT_UDS_ENDPOINT, DEFAULT_UDS_PATH};
pub use error::Error;
pub use value::DataValue;
