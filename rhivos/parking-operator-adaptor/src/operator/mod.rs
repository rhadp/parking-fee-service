//! PARKING_OPERATOR client module.
//!
//! Exposes the [`OperatorApi`] trait, the [`OperatorClient`] HTTP
//! implementation, and the [`RetryOperatorClient`] wrapper.

pub mod client;
pub mod models;
pub mod retry;
mod traits;

pub use client::OperatorClient;
pub use retry::RetryOperatorClient;
pub use traits::OperatorApi;
