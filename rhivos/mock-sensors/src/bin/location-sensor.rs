fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        eprintln!("Usage: location-sensor");
        eprintln!("  RHIVOS location sensor mock");
        std::process::exit(1);
    }
    println!("location-sensor v0.1.0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
