//! VSS signal path constants used by services across the workspace.
//!
//! All VSS signal paths are defined here to prevent duplication and ensure
//! consistency between the locking service, mock sensors, and tests.

/// Driver-side door locked state.
pub const DOOR_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";

/// Driver-side door open state.
pub const DOOR_IS_OPEN: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen";

/// Vehicle speed in km/h.
pub const SPEED: &str = "Vehicle.Speed";

/// Vehicle current latitude.
pub const LOCATION_LAT: &str = "Vehicle.CurrentLocation.Latitude";

/// Vehicle current longitude.
pub const LOCATION_LON: &str = "Vehicle.CurrentLocation.Longitude";

/// Lock/unlock command request. `true` = lock, `false` = unlock.
pub const COMMAND_DOOR_LOCK: &str = "Vehicle.Command.Door.Lock";

/// Result of the last lock command processed by the locking service.
pub const LOCK_RESULT: &str = "Vehicle.Command.Door.LockResult";

/// Whether a parking session is currently active.
pub const PARKING_SESSION_ACTIVE: &str = "Vehicle.Parking.SessionActive";
