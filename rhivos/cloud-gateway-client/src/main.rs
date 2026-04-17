fn main() {
    for arg in std::env::args().skip(1) {
        if arg.starts_with('-') {
            eprintln!("Usage: cloud-gateway-client");
            std::process::exit(1);
        }
    }
    println!("cloud-gateway-client v0.1.0");
}

#[cfg(test)]
mod tests {
    /// Verifies the crate compiles successfully (01-REQ-8.1, TS-01-26).
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
