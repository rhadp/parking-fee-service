//! Mock sensor CLI tool.
//!
//! Publishes mock VSS signal values to the Eclipse Kuksa Databroker via gRPC.
//! Supports setting: location (lat/lon), speed, and door status.

use clap::{Parser, Subcommand};
use tonic::transport::Channel;
use tracing::{error, info};

/// Generated Kuksa VAL v1 gRPC types (vendored proto).
mod kuksa {
    pub mod val {
        pub mod v1 {
            tonic::include_proto!("kuksa.val.v1");
        }
    }
}

use kuksa::val::v1::{
    val_client::ValClient, DataEntry, Datapoint, EntryUpdate, Field, SetRequest,
};

/// Mock sensor CLI — publishes VSS signals to Kuksa Databroker.
#[derive(Parser, Debug)]
#[command(name = "mock-sensors", about = "Publish mock VSS signals to Kuksa Databroker")]
struct Args {
    /// Address of the Kuksa Databroker.
    #[arg(long, env = "DATABROKER_ADDR", default_value = "http://localhost:55555")]
    databroker_addr: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::enum_variant_names)] // Prefix matches CLI subcommand naming convention (set-*)
enum Command {
    /// Set Vehicle.CurrentLocation (latitude, longitude).
    SetLocation {
        /// Latitude value.
        #[arg(long)]
        lat: f64,
        /// Longitude value.
        #[arg(long)]
        lon: f64,
    },
    /// Set Vehicle.Speed.
    SetSpeed {
        /// Speed value in km/h.
        #[arg(long)]
        value: f64,
    },
    /// Set Vehicle.Cabin.Door.Row1.DriverSide.IsOpen.
    SetDoor {
        /// Whether the door is open (true/false).
        #[arg(long, num_args = 1)]
        open: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("Connecting to Kuksa Databroker at {}", args.databroker_addr);

    let channel = Channel::from_shared(args.databroker_addr.clone())
        .map_err(|e| {
            error!("Invalid databroker address '{}': {}", args.databroker_addr, e);
            e
        })?
        .connect()
        .await
        .map_err(|e| {
            error!(
                "Failed to connect to Kuksa Databroker at '{}': {}",
                args.databroker_addr, e
            );
            e
        })?;

    let mut client = ValClient::new(channel);

    match args.command {
        Command::SetLocation { lat, lon } => {
            set_location(&mut client, lat, lon).await?;
        }
        Command::SetSpeed { value } => {
            set_speed(&mut client, value).await?;
        }
        Command::SetDoor { open } => {
            set_door(&mut client, open).await?;
        }
    }

    Ok(())
}

/// Build an EntryUpdate for a double-valued VSS signal.
fn double_entry_update(path: &str, value: f64) -> EntryUpdate {
    EntryUpdate {
        entry: Some(DataEntry {
            path: path.to_string(),
            value: Some(Datapoint {
                timestamp: None,
                value: Some(kuksa::val::v1::datapoint::Value::Double(value)),
            }),
        }),
        fields: vec![Field::Value as i32],
    }
}

/// Build an EntryUpdate for a bool-valued VSS signal.
fn bool_entry_update(path: &str, value: bool) -> EntryUpdate {
    EntryUpdate {
        entry: Some(DataEntry {
            path: path.to_string(),
            value: Some(Datapoint {
                timestamp: None,
                value: Some(kuksa::val::v1::datapoint::Value::Bool(value)),
            }),
        }),
        fields: vec![Field::Value as i32],
    }
}

async fn set_location(
    client: &mut ValClient<Channel>,
    lat: f64,
    lon: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let updates = vec![
        double_entry_update("Vehicle.CurrentLocation.Latitude", lat),
        double_entry_update("Vehicle.CurrentLocation.Longitude", lon),
    ];

    let resp = client
        .set(SetRequest { updates })
        .await
        .map_err(|e| {
            error!("SetLocation RPC failed: {}", e);
            e
        })?;

    info!("SetLocation response: {:?}", resp.into_inner());
    Ok(())
}

async fn set_speed(
    client: &mut ValClient<Channel>,
    value: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let updates = vec![double_entry_update("Vehicle.Speed", value)];

    let resp = client
        .set(SetRequest { updates })
        .await
        .map_err(|e| {
            error!("SetSpeed RPC failed: {}", e);
            e
        })?;

    info!("SetSpeed response: {:?}", resp.into_inner());
    Ok(())
}

async fn set_door(
    client: &mut ValClient<Channel>,
    open: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let updates = vec![bool_entry_update(
        "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen",
        open,
    )];

    let resp = client
        .set(SetRequest { updates })
        .await
        .map_err(|e| {
            error!("SetDoor RPC failed: {}", e);
            e
        })?;

    info!("SetDoor response: {:?}", resp.into_inner());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn cli_parses_set_location() {
        let args = Args::parse_from([
            "mock-sensors",
            "set-location",
            "--lat",
            "48.8566",
            "--lon",
            "2.3522",
        ]);
        assert_eq!(args.databroker_addr, "http://localhost:55555");
        match args.command {
            Command::SetLocation { lat, lon } => {
                assert!((lat - 48.8566).abs() < f64::EPSILON);
                assert!((lon - 2.3522).abs() < f64::EPSILON);
            }
            _ => panic!("Expected SetLocation command"),
        }
    }

    #[test]
    fn cli_parses_set_speed() {
        let args = Args::parse_from(["mock-sensors", "set-speed", "--value", "60.5"]);
        match args.command {
            Command::SetSpeed { value } => {
                assert!((value - 60.5).abs() < f64::EPSILON);
            }
            _ => panic!("Expected SetSpeed command"),
        }
    }

    #[test]
    fn cli_parses_set_door_open() {
        let args = Args::parse_from(["mock-sensors", "set-door", "--open", "true"]);
        match args.command {
            Command::SetDoor { open } => assert!(open),
            _ => panic!("Expected SetDoor command"),
        }
    }

    #[test]
    fn cli_parses_set_door_closed() {
        let args = Args::parse_from(["mock-sensors", "set-door", "--open", "false"]);
        match args.command {
            Command::SetDoor { open } => assert!(!open),
            _ => panic!("Expected SetDoor command"),
        }
    }

    #[test]
    fn cli_parses_custom_databroker_addr() {
        let args = Args::parse_from([
            "mock-sensors",
            "--databroker-addr",
            "http://192.168.1.100:55555",
            "set-speed",
            "--value",
            "30.0",
        ]);
        assert_eq!(args.databroker_addr, "http://192.168.1.100:55555");
    }

    #[test]
    fn double_entry_update_builds_correctly() {
        let update = double_entry_update("Vehicle.Speed", 42.0);
        let entry = update.entry.unwrap();
        assert_eq!(entry.path, "Vehicle.Speed");
        let dp = entry.value.unwrap();
        match dp.value {
            Some(kuksa::val::v1::datapoint::Value::Double(v)) => {
                assert!((v - 42.0).abs() < f64::EPSILON);
            }
            _ => panic!("Expected double value"),
        }
        assert_eq!(update.fields, vec![Field::Value as i32]);
    }

    #[test]
    fn bool_entry_update_builds_correctly() {
        let update = bool_entry_update("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", true);
        let entry = update.entry.unwrap();
        assert_eq!(entry.path, "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen");
        let dp = entry.value.unwrap();
        match dp.value {
            Some(kuksa::val::v1::datapoint::Value::Bool(v)) => assert!(v),
            _ => panic!("Expected bool value"),
        }
    }
}
