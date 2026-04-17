fn main() {
    for arg in std::env::args().skip(1) {
        if arg.starts_with('-') {
            eprintln!("Usage: speed-sensor");
            std::process::exit(1);
        }
    }
    println!("speed-sensor v0.1.0");
}
