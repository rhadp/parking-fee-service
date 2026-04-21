//! parking-operator-adaptor library crate.
//!
//! Bridges the in-vehicle PARKING_APP (via gRPC) with a PARKING_OPERATOR
//! backend (via REST), and autonomously manages parking sessions based on
//! lock/unlock events from DATA_BROKER.

pub mod broker;
pub mod config;
pub mod event_loop;
pub mod operator;
pub mod proptest_cases;
pub mod session;
