//! LOCKING_SERVICE — ASIL-B rated Rust service (RHIVOS safety partition).
//!
//! Subscribes to `Vehicle.Command.Door.Lock` from DATA_BROKER, validates safety
//! constraints (vehicle speed, door ajar), manages the lock state, and publishes
//! command responses.  Full implementation is wired up in task group 3.

mod broker;
mod command;
mod config;
mod process;
mod response;
mod safety;

#[cfg(test)]
pub mod testing;

#[cfg(test)]
pub mod proptest_cases;

fn main() {
    for arg in std::env::args().skip(1) {
        if arg.starts_with('-') {
            eprintln!("Usage: locking-service");
            std::process::exit(1);
        }
    }
    println!("locking-service v0.1.0");
}

#[cfg(test)]
mod tests {
    /// Verifies the crate compiles successfully (01-REQ-8.1, TS-01-26).
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
