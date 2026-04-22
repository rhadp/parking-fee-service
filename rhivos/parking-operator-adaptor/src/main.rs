pub mod broker;
pub mod config;
pub mod event_loop;
pub mod operator;
pub mod session;
#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod proptest_cases;

use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1].starts_with('-') {
        eprintln!("Usage: parking-operator-adaptor");
        eprintln!("  RHIVOS parking operator adaptor skeleton.");
        process::exit(1);
    }
    println!("parking-operator-adaptor v0.1.0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        // Verify the binary compiles and modules are accessible.
        let _ = super::config::Config {
            parking_operator_url: String::new(),
            data_broker_addr: String::new(),
            grpc_port: 0,
            vehicle_id: String::new(),
            zone_id: String::new(),
        };
    }
}
