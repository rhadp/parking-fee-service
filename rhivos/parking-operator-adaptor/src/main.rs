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
        assert!(true);
    }
}
