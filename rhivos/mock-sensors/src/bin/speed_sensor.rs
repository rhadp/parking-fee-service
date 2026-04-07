use mock_sensors::{publish_datapoint, DatapointValue};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut speed: Option<f64> = None;
    let mut broker_addr = std::env::var("DATABROKER_ADDR")
        .unwrap_or_else(|_| "http://localhost:55556".to_string());

    for arg in &args {
        if let Some(val) = arg.strip_prefix("--speed=") {
            match val.parse::<f64>() {
                Ok(v) => speed = Some(v),
                Err(_) => {
                    eprintln!("Error: invalid value for --speed: {}", val);
                    std::process::exit(1);
                }
            }
        } else if let Some(val) = arg.strip_prefix("--broker-addr=") {
            broker_addr = val.to_string();
        } else {
            eprintln!("Error: unrecognized argument: {}", arg);
            std::process::exit(1);
        }
    }

    let speed = speed.unwrap_or_else(|| {
        eprintln!("Error: --speed is required");
        std::process::exit(1);
    });

    if let Err(e) = publish_datapoint(
        &broker_addr,
        "Vehicle.Speed",
        DatapointValue::Float(speed as f32),
    )
    .await
    {
        eprintln!("Error: failed to publish speed: {}", e);
        std::process::exit(1);
    }

    println!("speed-sensor: published speed={}", speed);
}
