pub mod config;
pub mod grpc;
pub mod operator;
pub mod session;
pub mod broker;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("parking-operator-adaptor starting...");
}
