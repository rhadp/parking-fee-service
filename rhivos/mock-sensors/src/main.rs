use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1].starts_with('-') {
        eprintln!("Usage: mock-sensors");
        eprintln!("  RHIVOS mock sensors skeleton. Use location-sensor, speed-sensor, or door-sensor binaries.");
        process::exit(1);
    }
    println!("mock-sensors v0.1.0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
