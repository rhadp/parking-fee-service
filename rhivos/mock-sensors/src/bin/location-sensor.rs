fn main() {
    for arg in std::env::args().skip(1) {
        if arg.starts_with('-') {
            eprintln!("Usage: location-sensor");
            std::process::exit(1);
        }
    }
    println!("location-sensor v0.1.0");
}
