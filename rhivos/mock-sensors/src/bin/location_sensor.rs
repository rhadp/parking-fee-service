use mock_sensors::{publish_datapoint, DatapointValue};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut lat: Option<f64> = None;
    let mut lon: Option<f64> = None;
    let mut broker_addr = std::env::var("DATABROKER_ADDR")
        .unwrap_or_else(|_| "http://localhost:55556".to_string());

    for arg in &args {
        if let Some(val) = arg.strip_prefix("--lat=") {
            match val.parse::<f64>() {
                Ok(v) => lat = Some(v),
                Err(_) => {
                    eprintln!("Error: invalid value for --lat: {}", val);
                    std::process::exit(1);
                }
            }
        } else if let Some(val) = arg.strip_prefix("--lon=") {
            match val.parse::<f64>() {
                Ok(v) => lon = Some(v),
                Err(_) => {
                    eprintln!("Error: invalid value for --lon: {}", val);
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

    let lat = lat.unwrap_or_else(|| {
        eprintln!("Error: --lat is required");
        std::process::exit(1);
    });
    let lon = lon.unwrap_or_else(|| {
        eprintln!("Error: --lon is required");
        std::process::exit(1);
    });

    if let Err(e) = publish_datapoint(
        &broker_addr,
        "Vehicle.CurrentLocation.Latitude",
        DatapointValue::Double(lat),
    )
    .await
    {
        eprintln!("Error: failed to publish latitude: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = publish_datapoint(
        &broker_addr,
        "Vehicle.CurrentLocation.Longitude",
        DatapointValue::Double(lon),
    )
    .await
    {
        eprintln!("Error: failed to publish longitude: {}", e);
        std::process::exit(1);
    }

    println!("location-sensor: published lat={} lon={}", lat, lon);
}

// Suppress unused import warnings in test mode where tokio may not be in scope
#[cfg(test)]
mod tests {}
