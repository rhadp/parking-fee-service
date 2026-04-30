pub mod adapter;
pub mod config;
pub mod install;
pub mod monitor;
pub mod offload;
pub mod podman;
pub mod state;

#[cfg(test)]
pub mod proptest_cases;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        eprintln!("Usage: {}", args[0]);
        eprintln!("Error: unknown argument '{}'", args[1]);
        std::process::exit(1);
    }
    println!("update-service v0.1.0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        // Placeholder test: verifies the crate compiles.
    }
}
