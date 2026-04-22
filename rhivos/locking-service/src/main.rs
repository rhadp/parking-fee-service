pub mod broker;
pub mod command;
pub mod config;
pub mod process;
pub mod response;
pub mod safety;

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod proptest_cases;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1].starts_with('-') {
        eprintln!("Usage: locking-service");
        eprintln!("  RHIVOS locking service skeleton.");
        std::process::exit(1);
    }
    println!("locking-service v0.1.0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
