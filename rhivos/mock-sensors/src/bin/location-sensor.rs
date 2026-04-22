use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1].starts_with('-') {
        eprintln!("Usage: location-sensor [--lat=<lat>] [--lon=<lon>] [--broker-addr=<addr>]");
        eprintln!("  RHIVOS location sensor mock.");
        process::exit(1);
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
