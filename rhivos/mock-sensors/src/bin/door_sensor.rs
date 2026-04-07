use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        eprintln!("Usage: door-sensor");
        eprintln!("Error: unrecognized arguments: {:?}", &args[1..]);
        process::exit(1);
    }
    println!("door-sensor v0.1.0");
}
