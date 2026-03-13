pub mod broker;
pub mod command;
pub mod config;
pub mod process;
pub mod response;
pub mod safety;

#[cfg(test)]
pub mod proptest_cases;
#[cfg(test)]
pub mod testing;

fn main() {
    println!("locking-service v0.1.0 - Vehicle door locking service");
    println!();
    println!("Usage: locking-service [options]");
    println!();
    println!("This is a skeleton implementation. See spec 03 for full functionality.");
}
