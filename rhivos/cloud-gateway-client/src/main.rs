// Task group 1: modules declared with stub implementations.
// Dead-code warnings are expected until implementation is complete (task groups 2–5).
#![allow(dead_code)]

mod broker;
mod command;
mod config;
mod nats_client;
mod proptest_cases;
mod relay;
mod telemetry;
mod testing;

fn main() {
    println!("cloud-gateway-client v0.1.0 - Vehicle-to-cloud gateway client");
    println!();
    println!("Usage: cloud-gateway-client");
    println!();
    println!("Environment:");
    println!("  VIN              Vehicle identification number (required)");
    println!("  NATS_URL         NATS server URL (default: nats://localhost:4222)");
    println!("  DATABROKER_ADDR  DATA_BROKER gRPC address (default: http://localhost:55556)");
    println!("  BEARER_TOKEN     Expected bearer token for command auth (default: demo-token)");
}
