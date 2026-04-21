fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Require "serve" sub-command to start the service.
    if args.get(1).map(|s| s.as_str()) == Some("serve") {
        println!("cloud-gateway-client v0.1.0 starting (not yet implemented)");
        return;
    }
    // Unknown flags → print usage and exit 1.
    for arg in &args[1..] {
        if arg.starts_with('-') {
            eprintln!("usage: cloud-gateway-client serve");
            std::process::exit(1);
        }
    }
    // No args or unrecognised sub-command → print usage and exit 0.
    println!("usage: cloud-gateway-client serve");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
