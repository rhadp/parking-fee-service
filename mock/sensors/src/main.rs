//! Mock sensor CLI tool.
//!
//! Publishes mock VSS signal values to the Eclipse Kuksa Databroker via gRPC.
//! Uses the shared `parking-proto` crate for Kuksa client and signal constants.
//!
//! # Subcommands
//!
//! - `set-location <lat> <lon>` — write latitude/longitude
//! - `set-speed <value>` — write vehicle speed (km/h)
//! - `set-door <open|closed>` — write door-open state
//! - `lock-command <lock|unlock>` — write lock/unlock command

use std::process;

use clap::{Parser, Subcommand};
use parking_proto::kuksa_client::KuksaClient;
use parking_proto::signals;
use tracing::{error, info};

/// Mock sensor CLI — publishes VSS signals to Kuksa Databroker.
#[derive(Parser, Debug)]
#[command(name = "mock-sensors", about = "Publish mock VSS signals to Kuksa Databroker")]
struct Args {
    /// Address of the Kuksa Databroker (e.g. http://localhost:55555).
    #[arg(long, env = "DATABROKER_ADDR", default_value = "http://localhost:55555")]
    databroker_addr: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug, Clone, PartialEq)]
#[allow(clippy::enum_variant_names)] // CLI subcommand naming: set-location, set-speed, set-door, lock-command
enum Command {
    /// Set Vehicle.CurrentLocation (latitude, longitude).
    SetLocation {
        /// Latitude value.
        lat: f64,
        /// Longitude value.
        lon: f64,
    },
    /// Set Vehicle.Speed (km/h).
    SetSpeed {
        /// Speed value in km/h.
        value: f64,
    },
    /// Set Vehicle.Cabin.Door.Row1.DriverSide.IsOpen.
    SetDoor {
        /// Door state: "open" or "closed".
        state: DoorState,
    },
    /// Set Vehicle.Command.Door.Lock (lock/unlock command).
    LockCommand {
        /// Lock action: "lock" or "unlock".
        action: LockAction,
    },
}

/// Door state argument: `open` or `closed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoorState {
    Open,
    Closed,
}

impl std::str::FromStr for DoorState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(DoorState::Open),
            "closed" => Ok(DoorState::Closed),
            _ => Err(format!(
                "invalid door state '{}': expected 'open' or 'closed'",
                s
            )),
        }
    }
}

impl std::fmt::Display for DoorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DoorState::Open => write!(f, "open"),
            DoorState::Closed => write!(f, "closed"),
        }
    }
}

/// Lock action argument: `lock` or `unlock`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockAction {
    Lock,
    Unlock,
}

impl std::str::FromStr for LockAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "lock" => Ok(LockAction::Lock),
            "unlock" => Ok(LockAction::Unlock),
            _ => Err(format!(
                "invalid lock action '{}': expected 'lock' or 'unlock'",
                s
            )),
        }
    }
}

impl std::fmt::Display for LockAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockAction::Lock => write!(f, "lock"),
            LockAction::Unlock => write!(f, "unlock"),
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    if let Err(e) = run(args).await {
        error!("{e}");
        process::exit(1);
    }
}

/// Execute the CLI command. Separated from `main` for testability and clean
/// error handling: connection failures and gRPC errors are propagated as
/// `Box<dyn Error>` and printed before exiting with a non-zero code.
async fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    info!("Connecting to Kuksa Databroker at {}", args.databroker_addr);

    let client = KuksaClient::connect(&args.databroker_addr).await.map_err(|e| {
        format!(
            "Failed to connect to Kuksa Databroker at '{}': {}",
            args.databroker_addr, e
        )
    })?;

    match args.command {
        Command::SetLocation { lat, lon } => {
            client
                .set_f64(signals::LOCATION_LAT, lat)
                .await
                .map_err(|e| format!("Failed to set latitude: {e}"))?;
            client
                .set_f64(signals::LOCATION_LON, lon)
                .await
                .map_err(|e| format!("Failed to set longitude: {e}"))?;
            info!(
                "Set location: {} = {}, {} = {}",
                signals::LOCATION_LAT, lat, signals::LOCATION_LON, lon
            );
        }
        Command::SetSpeed { value } => {
            client
                .set_f64(signals::SPEED, value)
                .await
                .map_err(|e| format!("Failed to set speed: {e}"))?;
            info!("Set speed: {} = {} km/h", signals::SPEED, value);
        }
        Command::SetDoor { state } => {
            let is_open = state == DoorState::Open;
            client
                .set_bool(signals::DOOR_IS_OPEN, is_open)
                .await
                .map_err(|e| format!("Failed to set door state: {e}"))?;
            info!(
                "Set door: {} = {} ({})",
                signals::DOOR_IS_OPEN, is_open, state
            );
        }
        Command::LockCommand { action } => {
            let lock_value = action == LockAction::Lock;
            client
                .set_bool(signals::COMMAND_DOOR_LOCK, lock_value)
                .await
                .map_err(|e| format!("Failed to set lock command: {e}"))?;
            info!(
                "Set lock command: {} = {} ({})",
                signals::COMMAND_DOOR_LOCK, lock_value, action
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    // ── CLI argument parsing tests ──────────────────────────────────────

    #[test]
    fn cli_parses_set_location() {
        let args = Args::parse_from(["mock-sensors", "set-location", "48.8566", "2.3522"]);
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
        let args = Args::parse_from(["mock-sensors", "set-speed", "60.5"]);
        match args.command {
            Command::SetSpeed { value } => {
                assert!((value - 60.5).abs() < f64::EPSILON);
            }
            _ => panic!("Expected SetSpeed command"),
        }
    }

    #[test]
    fn cli_parses_set_door_open() {
        let args = Args::parse_from(["mock-sensors", "set-door", "open"]);
        match args.command {
            Command::SetDoor { state } => assert_eq!(state, DoorState::Open),
            _ => panic!("Expected SetDoor command"),
        }
    }

    #[test]
    fn cli_parses_set_door_closed() {
        let args = Args::parse_from(["mock-sensors", "set-door", "closed"]);
        match args.command {
            Command::SetDoor { state } => assert_eq!(state, DoorState::Closed),
            _ => panic!("Expected SetDoor command"),
        }
    }

    #[test]
    fn cli_parses_lock_command_lock() {
        let args = Args::parse_from(["mock-sensors", "lock-command", "lock"]);
        match args.command {
            Command::LockCommand { action } => assert_eq!(action, LockAction::Lock),
            _ => panic!("Expected LockCommand command"),
        }
    }

    #[test]
    fn cli_parses_lock_command_unlock() {
        let args = Args::parse_from(["mock-sensors", "lock-command", "unlock"]);
        match args.command {
            Command::LockCommand { action } => assert_eq!(action, LockAction::Unlock),
            _ => panic!("Expected LockCommand command"),
        }
    }

    #[test]
    fn cli_parses_custom_databroker_addr() {
        let args = Args::parse_from([
            "mock-sensors",
            "--databroker-addr",
            "http://192.168.1.100:55555",
            "set-speed",
            "30.0",
        ]);
        assert_eq!(args.databroker_addr, "http://192.168.1.100:55555");
    }

    // ── DoorState parsing tests ─────────────────────────────────────────

    #[test]
    fn door_state_parse_open() {
        assert_eq!("open".parse::<DoorState>().unwrap(), DoorState::Open);
    }

    #[test]
    fn door_state_parse_closed() {
        assert_eq!("closed".parse::<DoorState>().unwrap(), DoorState::Closed);
    }

    #[test]
    fn door_state_parse_case_insensitive() {
        assert_eq!("OPEN".parse::<DoorState>().unwrap(), DoorState::Open);
        assert_eq!("Closed".parse::<DoorState>().unwrap(), DoorState::Closed);
    }

    #[test]
    fn door_state_parse_invalid() {
        let result = "ajar".parse::<DoorState>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ajar"));
    }

    #[test]
    fn door_state_display() {
        assert_eq!(DoorState::Open.to_string(), "open");
        assert_eq!(DoorState::Closed.to_string(), "closed");
    }

    // ── LockAction parsing tests ────────────────────────────────────────

    #[test]
    fn lock_action_parse_lock() {
        assert_eq!("lock".parse::<LockAction>().unwrap(), LockAction::Lock);
    }

    #[test]
    fn lock_action_parse_unlock() {
        assert_eq!("unlock".parse::<LockAction>().unwrap(), LockAction::Unlock);
    }

    #[test]
    fn lock_action_parse_case_insensitive() {
        assert_eq!("LOCK".parse::<LockAction>().unwrap(), LockAction::Lock);
        assert_eq!("Unlock".parse::<LockAction>().unwrap(), LockAction::Unlock);
    }

    #[test]
    fn lock_action_parse_invalid() {
        let result = "toggle".parse::<LockAction>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("toggle"));
    }

    #[test]
    fn lock_action_display() {
        assert_eq!(LockAction::Lock.to_string(), "lock");
        assert_eq!(LockAction::Unlock.to_string(), "unlock");
    }

    // ── Default address test ────────────────────────────────────────────

    #[test]
    fn default_databroker_addr() {
        let args = Args::parse_from(["mock-sensors", "set-speed", "0"]);
        assert_eq!(args.databroker_addr, "http://localhost:55555");
    }

    // ── Integration tests (require `make infra-up`) ─────────────────────

    #[tokio::test]
    #[ignore]
    async fn integration_set_location_roundtrip() {
        let addr = std::env::var("DATABROKER_ADDR")
            .unwrap_or_else(|_| "http://localhost:55555".to_string());

        let client = KuksaClient::connect(&addr)
            .await
            .expect("failed to connect to Kuksa Databroker (is `make infra-up` running?)");

        // Write location via the same path the CLI would use
        let lat = 48.8566;
        let lon = 2.3522;
        client.set_f64(signals::LOCATION_LAT, lat).await.unwrap();
        client.set_f64(signals::LOCATION_LON, lon).await.unwrap();

        // Read back
        let got_lat = client.get_f64(signals::LOCATION_LAT).await.unwrap();
        let got_lon = client.get_f64(signals::LOCATION_LON).await.unwrap();

        assert!(got_lat.is_some(), "expected latitude to be set");
        assert!(got_lon.is_some(), "expected longitude to be set");
        assert!(
            (got_lat.unwrap() - lat).abs() < 0.001,
            "latitude mismatch: got {:?}",
            got_lat
        );
        assert!(
            (got_lon.unwrap() - lon).abs() < 0.001,
            "longitude mismatch: got {:?}",
            got_lon
        );
    }

    #[tokio::test]
    #[ignore]
    async fn integration_set_speed_roundtrip() {
        let addr = std::env::var("DATABROKER_ADDR")
            .unwrap_or_else(|_| "http://localhost:55555".to_string());

        let client = KuksaClient::connect(&addr)
            .await
            .expect("failed to connect to Kuksa Databroker");

        let speed = 55.5;
        client.set_f64(signals::SPEED, speed).await.unwrap();

        // Speed is stored as float in VSS, may lose some precision
        let got = client.get_f32(signals::SPEED).await.unwrap();
        assert!(got.is_some(), "expected speed to be set");
        assert!(
            (f64::from(got.unwrap()) - speed).abs() < 0.5,
            "speed mismatch: got {:?}",
            got
        );
    }

    #[tokio::test]
    #[ignore]
    async fn integration_set_door_roundtrip() {
        let addr = std::env::var("DATABROKER_ADDR")
            .unwrap_or_else(|_| "http://localhost:55555".to_string());

        let client = KuksaClient::connect(&addr)
            .await
            .expect("failed to connect to Kuksa Databroker");

        // Set door open
        client.set_bool(signals::DOOR_IS_OPEN, true).await.unwrap();
        let got = client.get_bool(signals::DOOR_IS_OPEN).await.unwrap();
        assert_eq!(got, Some(true), "expected door to be open");

        // Set door closed
        client.set_bool(signals::DOOR_IS_OPEN, false).await.unwrap();
        let got = client.get_bool(signals::DOOR_IS_OPEN).await.unwrap();
        assert_eq!(got, Some(false), "expected door to be closed");
    }

    #[tokio::test]
    #[ignore]
    async fn integration_lock_command_roundtrip() {
        let addr = std::env::var("DATABROKER_ADDR")
            .unwrap_or_else(|_| "http://localhost:55555".to_string());

        let client = KuksaClient::connect(&addr)
            .await
            .expect("failed to connect to Kuksa Databroker");

        // Send lock command
        client
            .set_bool(signals::COMMAND_DOOR_LOCK, true)
            .await
            .unwrap();
        let got = client.get_bool(signals::COMMAND_DOOR_LOCK).await.unwrap();
        assert_eq!(got, Some(true), "expected lock command = true");

        // Send unlock command
        client
            .set_bool(signals::COMMAND_DOOR_LOCK, false)
            .await
            .unwrap();
        let got = client.get_bool(signals::COMMAND_DOOR_LOCK).await.unwrap();
        assert_eq!(got, Some(false), "expected lock command = false");
    }
}
