pub mod broker;
pub mod config;
pub mod event_loop;
pub mod operator;
pub mod session;

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod proptest_cases;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        eprintln!("Usage: parking-operator-adaptor");
        eprintln!("  RHIVOS parking operator adaptor skeleton");
        std::process::exit(1);
    }
    println!("parking-operator-adaptor v0.1.0");
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
