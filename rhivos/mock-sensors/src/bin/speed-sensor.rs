fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        eprintln!("Usage: speed-sensor");
        eprintln!("  RHIVOS speed sensor mock");
        std::process::exit(1);
    }
    println!("speed-sensor v0.1.0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
