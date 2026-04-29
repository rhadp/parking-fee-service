pub mod broker;
pub mod command;
pub mod config;
pub mod process;
pub mod response;
pub mod safety;

#[cfg(test)]
pub mod proptest_cases;
#[cfg(test)]
pub mod testing;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        eprintln!("Usage: {}", args[0]);
        eprintln!("Error: unknown argument '{}'", args[1]);
        std::process::exit(1);
    }
    println!("locking-service v0.1.0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        // Placeholder test: verifies the crate compiles.
    }
}
