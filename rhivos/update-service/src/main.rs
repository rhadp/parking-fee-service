pub mod adapter;
pub mod config;
pub mod grpc;
pub mod monitor;
pub mod offload;
pub mod podman;
pub mod state;

#[cfg(test)]
mod proptest_cases;

#[cfg(test)]
mod tests_install;

#[cfg(test)]
mod tests_lifecycle;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        eprintln!("Usage: update-service");
        eprintln!("  RHIVOS update service skeleton");
        std::process::exit(1);
    }
    println!("update-service v0.1.0");
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
