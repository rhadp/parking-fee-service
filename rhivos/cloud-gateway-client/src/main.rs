fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        eprintln!("Usage: cloud-gateway-client");
        eprintln!("  RHIVOS cloud gateway client skeleton");
        std::process::exit(1);
    }
    println!("cloud-gateway-client v0.1.0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
