fn main() {
    let args: Vec<String> = std::env::args().collect();
    for arg in &args[1..] {
        if arg.starts_with('-') {
            eprintln!("Usage: cloud-gateway-client");
            std::process::exit(1);
        }
    }
    println!("cloud-gateway-client v0.1.0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
