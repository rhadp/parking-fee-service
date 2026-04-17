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
