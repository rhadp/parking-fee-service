fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        eprintln!("Usage: mock-sensors");
        eprintln!("  RHIVOS mock sensors skeleton");
        std::process::exit(1);
    }
    println!("mock-sensors v0.1.0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
