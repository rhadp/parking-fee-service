//! PARKING_OPERATOR_ADAPTOR library crate.
//!
//! Bridges the PARKING_APP (gRPC) with the PARKING_OPERATOR backend (REST)
//! and manages parking sessions autonomously via DATA_BROKER lock/unlock events.

pub mod broker;
pub mod config;
pub mod event_loop;
pub mod grpc_server;
pub mod operator;
pub mod proto;
pub mod session;

#[cfg(test)]
pub mod proptest_cases;
