use mock_sensors::{publish_datapoint, DatapointValue};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut is_open: Option<bool> = None;
    let mut broker_addr = std::env::var("DATABROKER_ADDR")
        .unwrap_or_else(|_| "http://localhost:55556".to_string());

    for arg in args.iter() {
        if arg == "--open" {
            is_open = Some(true);
        } else if arg == "--closed" {
            is_open = Some(false);
        } else if let Some(val) = arg.strip_prefix("--broker-addr=") {
            broker_addr = val.to_string();
        } else {
            eprintln!("Error: unrecognized argument: {}", arg);
            std::process::exit(1);
        }
    }

    let is_open = is_open.unwrap_or_else(|| {
        eprintln!("Error: either --open or --closed is required");
        std::process::exit(1);
    });

    if let Err(e) = publish_datapoint(
        &broker_addr,
        "Vehicle.Cabin.Door.Row1.Left.IsOpen",
        DatapointValue::Bool(is_open),
    )
    .await
    {
        eprintln!("Error: failed to publish door state: {}", e);
        std::process::exit(1);
    }

    println!("door-sensor: published is_open={}", is_open);
}
