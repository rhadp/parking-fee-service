/// mock-sensors entry point.
/// The three sensor binaries (location-sensor, speed-sensor, door-sensor)
/// are declared as separate [[bin]] targets in Cargo.toml.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    for arg in &args[1..] {
        if arg.starts_with('-') {
            eprintln!("usage: mock-sensors");
            std::process::exit(1);
        }
    }
    println!("mock-sensors v0.1.0");
}
