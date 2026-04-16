//! LOCKING_SERVICE — ASIL-B rated door lock control service.
//!
//! Subscribes to `Vehicle.Command.Door.Lock` from DATA_BROKER, validates
//! safety constraints (speed, door ajar), manages lock state, and publishes
//! responses to `Vehicle.Command.Door.Response`.
//!
//! Usage: locking-service serve

// Suppress dead_code during TDD skeleton phase; remove once all modules are wired.
#![allow(dead_code)]

pub mod broker;
pub mod command;
pub mod config;
pub mod process;
pub mod response;
pub mod safety;

#[cfg(test)]
pub mod testing;
#[cfg(test)]
pub mod proptest_cases;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    for arg in &args[1..] {
        if arg.starts_with('-') {
            eprintln!("Usage: locking-service serve");
            std::process::exit(1);
        }
    }

    if args.get(1).map(|s| s.as_str()) == Some("serve") {
        // Full service startup implemented in task group 3.
        todo!("Implement service main loop in task group 3")
    }

    println!("locking-service v{}", env!("CARGO_PKG_VERSION"));
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
