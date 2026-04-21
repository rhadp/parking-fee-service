fn main() {
    let args: Vec<String> = std::env::args().collect();
    for arg in &args[1..] {
        if arg.starts_with('-') {
            eprintln!("usage: parking-operator-adaptor");
            std::process::exit(1);
        }
    }
    println!("parking-operator-adaptor v0.1.0");
}
