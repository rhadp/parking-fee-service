fn main() {
    let args: Vec<String> = std::env::args().collect();
    for arg in &args[1..] {
        if arg.starts_with('-') {
            eprintln!("Usage: door-sensor");
            std::process::exit(1);
        }
    }
    println!("door-sensor v0.1.0");
}
