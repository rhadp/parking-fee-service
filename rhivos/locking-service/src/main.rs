// Stub modules: types and functions defined for tests; implementation follows
// in task groups 2 and 3. The allow(dead_code) will be removed once main.rs
// wires these modules into the service entry point.
#[allow(dead_code)]
mod broker;
#[allow(dead_code)]
mod command;
#[allow(dead_code)]
mod config;
#[allow(dead_code)]
mod process;
#[allow(dead_code)]
mod response;
#[allow(dead_code)]
mod safety;

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod proptest_cases;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        eprintln!("Usage: locking-service");
        eprintln!("  RHIVOS locking service skeleton");
        std::process::exit(1);
    }
    println!("locking-service v0.1.0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        // Verify the binary crate compiles successfully.
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
    }
}
