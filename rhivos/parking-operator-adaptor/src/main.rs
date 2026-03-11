pub mod config;
pub mod grpc;
pub mod operator;
pub mod session;

#[tokio::main]
async fn main() {
    println!("parking-operator-adaptor starting...");
}
