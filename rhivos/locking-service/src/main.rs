pub mod command;
pub mod config;
pub mod safety;

fn main() {
    println!("locking-service starting...");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_startup() {
        assert!(true, "locking-service skeleton compiles and runs");
    }
}
