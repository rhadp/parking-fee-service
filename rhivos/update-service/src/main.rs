use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1].starts_with('-') {
        eprintln!("Usage: update-service");
        eprintln!("  RHIVOS update service skeleton.");
        process::exit(1);
    }
    println!("update-service v0.1.0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
