//! PARKING_OPERATOR_ADAPTOR — binary entry point.
//!
//! All application logic lives in the library crate (`lib.rs` and its
//! sub-modules).  This file is intentionally thin; task group 5 wires
//! everything together here.

fn main() {
    println!(
        "parking-operator-adaptor v{} starting",
        env!("CARGO_PKG_VERSION")
    );
    // Task group 5 implements the full async main (tokio runtime, gRPC server,
    // DATA_BROKER connection, shutdown handler).
}
