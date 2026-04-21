//! DATA_BROKER gRPC client for the CLOUD_GATEWAY_CLIENT.
//!
//! Wraps the kuksa.val.v1 VALService to provide:
//! - Writing command signals to DATA_BROKER ([04-REQ-6.3])
//! - Subscribing to `Vehicle.Command.Door.Response` for response relay ([04-REQ-3.3])
//! - Subscribing to telemetry signals for aggregation and publish ([04-REQ-3.2])

use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::errors::BrokerError;
use crate::models::SignalUpdate;

// ── VSS signal path constants ────────────────────────────────────────────────

/// Command actuator written by CLOUD_GATEWAY_CLIENT; consumed by LOCKING_SERVICE.
pub const SIGNAL_COMMAND: &str = "Vehicle.Command.Door.Lock";
/// Response signal written by LOCKING_SERVICE; relayed to NATS.
pub const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";
/// Telemetry: door lock state.
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
/// Telemetry: GPS latitude.
pub const SIGNAL_LATITUDE: &str = "Vehicle.CurrentLocation.Latitude";
/// Telemetry: GPS longitude.
pub const SIGNAL_LONGITUDE: &str = "Vehicle.CurrentLocation.Longitude";
/// Telemetry: parking session active flag.
pub const SIGNAL_PARKING_ACTIVE: &str = "Vehicle.Parking.SessionActive";

// ── Proto module ─────────────────────────────────────────────────────────────

mod proto {
    pub mod kuksa {
        pub mod val {
            pub mod v1 {
                tonic::include_proto!("kuksa.val.v1");
            }
        }
    }
}

use proto::kuksa::val::v1::{
    datapoint, val_service_client::ValServiceClient, DataEntry, Datapoint, EntryUpdate, Field,
    SetRequest, SubscribeEntry, SubscribeRequest,
};

// ── BrokerClient ─────────────────────────────────────────────────────────────

/// DATA_BROKER gRPC client managing signal writes and subscriptions.
pub struct BrokerClient {
    client: ValServiceClient<tonic::transport::Channel>,
}

impl BrokerClient {
    /// Establish a gRPC connection to DATA_BROKER at the address from `config`.
    ///
    /// Returns `Err(BrokerError::ConnectionFailed)` if the connection cannot
    /// be established. The caller (main) is responsible for exiting on failure.
    ///
    /// Validates: [04-REQ-3.1], [04-REQ-3.E1]
    pub async fn connect(config: &Config) -> Result<Self, BrokerError> {
        let addr = config.databroker_addr.clone();
        info!(addr = %addr, "Connecting to DATA_BROKER");

        match ValServiceClient::connect(addr.clone()).await {
            Ok(client) => {
                info!(addr = %addr, "Connected to DATA_BROKER");
                Ok(BrokerClient { client })
            }
            Err(e) => {
                error!(addr = %addr, error = %e, "Failed to connect to DATA_BROKER");
                Err(BrokerError::ConnectionFailed(e.to_string()))
            }
        }
    }

    /// Write a command payload string to `Vehicle.Command.Door.Lock` in DATA_BROKER.
    ///
    /// The payload is written verbatim without modification (Property 3: passthrough
    /// fidelity). Errors are returned so the caller can log and continue.
    ///
    /// Validates: [04-REQ-6.3]
    pub async fn write_command(&self, payload: &str) -> Result<(), BrokerError> {
        debug!(signal = SIGNAL_COMMAND, "Writing command to DATA_BROKER");

        let request = SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: SIGNAL_COMMAND.to_string(),
                    value: Some(Datapoint {
                        value: Some(datapoint::Value::String(payload.to_string())),
                    }),
                }),
                fields: vec![Field::Value as i32],
            }],
        };

        let mut client = self.client.clone();
        let response = client
            .set(request)
            .await
            .map_err(|e| {
                error!(signal = SIGNAL_COMMAND, error = %e, "Failed to write command to DATA_BROKER");
                BrokerError::WriteFailed(e.to_string())
            })?
            .into_inner();

        for entry_err in &response.errors {
            warn!(
                signal = SIGNAL_COMMAND,
                error = ?entry_err.error,
                "DATA_BROKER per-entry error on command write"
            );
        }

        info!(signal = SIGNAL_COMMAND, "Command forwarded to DATA_BROKER");
        Ok(())
    }

    /// Subscribe to `Vehicle.Command.Door.Response` and return a channel receiver
    /// that yields raw JSON string values as they arrive.
    ///
    /// A background task reads the gRPC stream and forwards values. If a value is
    /// not valid JSON, the task logs an error and skips it ([04-REQ-7.E1]).
    ///
    /// Validates: [04-REQ-3.3], [04-REQ-7.1], [04-REQ-7.E1]
    pub async fn subscribe_responses(&self) -> Result<mpsc::Receiver<String>, BrokerError> {
        let request = SubscribeRequest {
            entries: vec![SubscribeEntry {
                path: SIGNAL_RESPONSE.to_string(),
                fields: vec![Field::Value as i32],
            }],
        };

        let mut client = self.client.clone();
        let mut stream = client
            .subscribe(request)
            .await
            .map_err(|e| {
                error!(signal = SIGNAL_RESPONSE, error = %e, "Failed to subscribe to command response signal");
                BrokerError::SubscribeFailed(e.to_string())
            })?
            .into_inner();

        let (tx, rx) = mpsc::channel::<String>(64);

        tokio::spawn(async move {
            loop {
                match stream.message().await {
                    Ok(Some(response)) => {
                        for update in response.updates {
                            if let Some(DataEntry {
                                value: Some(Datapoint { value: Some(datapoint::Value::String(s)) }),
                                ..
                            }) = update.entry
                            {
                                // Validate JSON before forwarding ([04-REQ-7.E1]).
                                if serde_json::from_str::<serde_json::Value>(&s).is_err() {
                                    error!(
                                        signal = SIGNAL_RESPONSE,
                                        value = %s,
                                        "Response value from DATA_BROKER is not valid JSON; skipping"
                                    );
                                    continue;
                                }
                                if tx.send(s).await.is_err() {
                                    // Receiver dropped — exit the background task.
                                    return;
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        info!(signal = SIGNAL_RESPONSE, "Response subscribe stream ended");
                        return;
                    }
                    Err(e) => {
                        error!(signal = SIGNAL_RESPONSE, error = %e, "Response subscribe stream error");
                        return;
                    }
                }
            }
        });

        info!(signal = SIGNAL_RESPONSE, "Subscribed to command response signal");
        Ok(rx)
    }

    /// Subscribe to the four telemetry signals and return a channel receiver that
    /// yields parsed `SignalUpdate` values as they arrive.
    ///
    /// A single gRPC stream covers all four signals:
    /// - `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` (bool)
    /// - `Vehicle.CurrentLocation.Latitude` (double)
    /// - `Vehicle.CurrentLocation.Longitude` (double)
    /// - `Vehicle.Parking.SessionActive` (bool)
    ///
    /// Unknown or unparseable signal updates are logged at DEBUG level and skipped.
    ///
    /// Validates: [04-REQ-3.2]
    pub async fn subscribe_telemetry(&self) -> Result<mpsc::Receiver<SignalUpdate>, BrokerError> {
        let request = SubscribeRequest {
            entries: vec![
                SubscribeEntry {
                    path: SIGNAL_IS_LOCKED.to_string(),
                    fields: vec![Field::Value as i32],
                },
                SubscribeEntry {
                    path: SIGNAL_LATITUDE.to_string(),
                    fields: vec![Field::Value as i32],
                },
                SubscribeEntry {
                    path: SIGNAL_LONGITUDE.to_string(),
                    fields: vec![Field::Value as i32],
                },
                SubscribeEntry {
                    path: SIGNAL_PARKING_ACTIVE.to_string(),
                    fields: vec![Field::Value as i32],
                },
            ],
        };

        let mut client = self.client.clone();
        let mut stream = client
            .subscribe(request)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to subscribe to telemetry signals");
                BrokerError::SubscribeFailed(e.to_string())
            })?
            .into_inner();

        let (tx, rx) = mpsc::channel::<SignalUpdate>(128);

        tokio::spawn(async move {
            loop {
                match stream.message().await {
                    Ok(Some(response)) => {
                        for update in response.updates {
                            if let Some(DataEntry {
                                path,
                                value: Some(ref dp),
                            }) = update.entry
                            {
                                match parse_signal_update(&path, dp) {
                                    Some(signal_update) => {
                                        if tx.send(signal_update).await.is_err() {
                                            // Receiver dropped — exit gracefully.
                                            return;
                                        }
                                    }
                                    None => {
                                        debug!(
                                            path = %path,
                                            "Received unrecognised or unparseable telemetry signal; skipping"
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        info!("Telemetry subscribe stream ended");
                        return;
                    }
                    Err(e) => {
                        error!(error = %e, "Telemetry subscribe stream error");
                        return;
                    }
                }
            }
        });

        info!("Subscribed to telemetry signals");
        Ok(rx)
    }
}

// ── Signal update parsing ─────────────────────────────────────────────────────

/// Parse a raw DATA_BROKER `Datapoint` for a given VSS signal path into a typed
/// `SignalUpdate`.
///
/// Returns `None` for unrecognised signal paths or type mismatches.
/// This is a pure function with no I/O, unit-tested in isolation.
pub fn parse_signal_update(path: &str, dp: &Datapoint) -> Option<SignalUpdate> {
    match (path, dp.value.as_ref()?) {
        (SIGNAL_IS_LOCKED, datapoint::Value::Bool(v)) => Some(SignalUpdate::IsLocked(*v)),
        (SIGNAL_LATITUDE, datapoint::Value::Double(v)) => Some(SignalUpdate::Latitude(*v)),
        (SIGNAL_LONGITUDE, datapoint::Value::Double(v)) => Some(SignalUpdate::Longitude(*v)),
        (SIGNAL_PARKING_ACTIVE, datapoint::Value::Bool(v)) => Some(SignalUpdate::ParkingActive(*v)),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bool_dp(v: bool) -> Datapoint {
        Datapoint {
            value: Some(datapoint::Value::Bool(v)),
        }
    }

    fn make_double_dp(v: f64) -> Datapoint {
        Datapoint {
            value: Some(datapoint::Value::Double(v)),
        }
    }

    // ── parse_signal_update: IsLocked ────────────────────────────────────────

    #[test]
    fn parse_is_locked_true() {
        let dp = make_bool_dp(true);
        let result = parse_signal_update(SIGNAL_IS_LOCKED, &dp);
        assert!(
            matches!(result, Some(SignalUpdate::IsLocked(true))),
            "expected IsLocked(true), got {:?}", result
        );
    }

    #[test]
    fn parse_is_locked_false() {
        let dp = make_bool_dp(false);
        let result = parse_signal_update(SIGNAL_IS_LOCKED, &dp);
        assert!(
            matches!(result, Some(SignalUpdate::IsLocked(false))),
            "expected IsLocked(false), got {:?}", result
        );
    }

    // ── parse_signal_update: Latitude ────────────────────────────────────────

    #[test]
    fn parse_latitude_positive() {
        let dp = make_double_dp(48.1351);
        let result = parse_signal_update(SIGNAL_LATITUDE, &dp);
        match result {
            Some(SignalUpdate::Latitude(v)) => {
                assert!((v - 48.1351).abs() < 1e-9, "latitude value mismatch")
            }
            other => panic!("expected Latitude(48.1351), got {:?}", other),
        }
    }

    #[test]
    fn parse_latitude_negative() {
        let dp = make_double_dp(-33.8688);
        let result = parse_signal_update(SIGNAL_LATITUDE, &dp);
        match result {
            Some(SignalUpdate::Latitude(v)) => {
                assert!((v - (-33.8688)).abs() < 1e-9, "latitude value mismatch")
            }
            other => panic!("expected Latitude(-33.8688), got {:?}", other),
        }
    }

    // ── parse_signal_update: Longitude ───────────────────────────────────────

    #[test]
    fn parse_longitude_positive() {
        let dp = make_double_dp(11.582);
        let result = parse_signal_update(SIGNAL_LONGITUDE, &dp);
        match result {
            Some(SignalUpdate::Longitude(v)) => {
                assert!((v - 11.582).abs() < 1e-9, "longitude value mismatch")
            }
            other => panic!("expected Longitude(11.582), got {:?}", other),
        }
    }

    #[test]
    fn parse_longitude_negative() {
        let dp = make_double_dp(-122.4194);
        let result = parse_signal_update(SIGNAL_LONGITUDE, &dp);
        match result {
            Some(SignalUpdate::Longitude(v)) => {
                assert!((v - (-122.4194)).abs() < 1e-9, "longitude value mismatch")
            }
            other => panic!("expected Longitude(-122.4194), got {:?}", other),
        }
    }

    // ── parse_signal_update: ParkingActive ───────────────────────────────────

    #[test]
    fn parse_parking_active_true() {
        let dp = make_bool_dp(true);
        let result = parse_signal_update(SIGNAL_PARKING_ACTIVE, &dp);
        assert!(
            matches!(result, Some(SignalUpdate::ParkingActive(true))),
            "expected ParkingActive(true), got {:?}", result
        );
    }

    #[test]
    fn parse_parking_active_false() {
        let dp = make_bool_dp(false);
        let result = parse_signal_update(SIGNAL_PARKING_ACTIVE, &dp);
        assert!(
            matches!(result, Some(SignalUpdate::ParkingActive(false))),
            "expected ParkingActive(false), got {:?}", result
        );
    }

    // ── parse_signal_update: edge cases ──────────────────────────────────────

    #[test]
    fn parse_unknown_path_returns_none() {
        let dp = make_bool_dp(true);
        let result = parse_signal_update("Vehicle.Unknown.Signal", &dp);
        assert!(result.is_none(), "unknown path must return None");
    }

    #[test]
    fn parse_type_mismatch_returns_none() {
        // SIGNAL_LATITUDE expects Double, but provide Bool.
        let dp = make_bool_dp(true);
        let result = parse_signal_update(SIGNAL_LATITUDE, &dp);
        assert!(result.is_none(), "type mismatch must return None");
    }
}
