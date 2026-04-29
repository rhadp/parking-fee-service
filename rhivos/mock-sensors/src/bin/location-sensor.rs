fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        eprintln!("Usage: {}", args[0]);
        eprintln!("Error: unknown argument '{}'", args[1]);
        std::process::exit(1);
    }
    println!("location-sensor v0.1.0");
}
