fn main() {
    for arg in std::env::args().skip(1) {
        if arg.starts_with('-') {
            eprintln!("Usage: door-sensor");
            std::process::exit(1);
        }
    }
    println!("door-sensor v0.1.0");
}
