//! PARKING_OPERATOR_ADAPTOR — library root.
//!
//! Declares all modules so they are compiled for both the binary and the test
//! harness.  The binary entry point is [`main.rs`].

pub mod autonomous;
pub mod broker;
pub mod config;
pub mod grpc_service;
pub mod operator;
pub mod session;

#[cfg(test)]
pub mod testing;
