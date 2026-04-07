use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        eprintln!("Usage: parking-operator-adaptor");
        eprintln!("Error: unrecognized arguments: {:?}", &args[1..]);
        process::exit(1);
    }
    println!("parking-operator-adaptor v0.1.0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
