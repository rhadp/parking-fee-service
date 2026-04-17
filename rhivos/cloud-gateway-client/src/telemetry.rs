#![allow(dead_code)]

use crate::models::SignalUpdate;

/// Maintains current telemetry state and produces aggregated JSON payloads.
///
/// Signals that have never been observed are omitted from the serialized output
/// (REQ-8.3).  The state is updated via `update()` which returns a fresh JSON
/// string whenever any field changes.
pub struct TelemetryState {
    vin: String,
    is_locked: Option<bool>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    parking_active: Option<bool>,
}

impl TelemetryState {
    /// Create a new, empty telemetry state for the given VIN.
    pub fn new(vin: String) -> Self {
        TelemetryState {
            vin,
            is_locked: None,
            latitude: None,
            longitude: None,
            parking_active: None,
        }
    }

    /// Apply a signal update and return the aggregated telemetry JSON.
    ///
    /// Returns `Some(json)` with all currently-known field values, or `None`
    /// when the incoming value is identical to the previously stored value.
    pub fn update(&mut self, signal: SignalUpdate) -> Option<String> {
        let changed = match signal {
            SignalUpdate::IsLocked(v) => {
                let changed = self.is_locked != Some(v);
                self.is_locked = Some(v);
                changed
            }
            SignalUpdate::Latitude(v) => {
                // Use bitwise comparison for f64 to avoid NaN != NaN surprises
                let changed = self.latitude.map(|prev| prev.to_bits()) != Some(v.to_bits());
                self.latitude = Some(v);
                changed
            }
            SignalUpdate::Longitude(v) => {
                let changed = self.longitude.map(|prev| prev.to_bits()) != Some(v.to_bits());
                self.longitude = Some(v);
                changed
            }
            SignalUpdate::ParkingActive(v) => {
                let changed = self.parking_active != Some(v);
                self.parking_active = Some(v);
                changed
            }
        };

        if !changed {
            return None;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let msg = crate::models::TelemetryMessage {
            vin: self.vin.clone(),
            is_locked: self.is_locked,
            latitude: self.latitude,
            longitude: self.longitude,
            parking_active: self.parking_active,
            timestamp,
        };

        Some(
            serde_json::to_string(&msg)
                .expect("TelemetryMessage serialization must not fail"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SignalUpdate;

    // ------------------------------------------------------------------
    // TS-04-7: First update produces JSON; unset fields are omitted
    // ------------------------------------------------------------------

    /// TS-04-7: After a single IsLocked update, JSON contains vin + is_locked +
    /// timestamp, and omits latitude, longitude, parking_active.
    ///
    /// Validates [04-REQ-8.1], [04-REQ-8.2], [04-REQ-8.3]
    #[test]
    fn test_telemetry_first_update_is_locked() {
        let mut state = TelemetryState::new("VIN-001".to_string());
        let result = state.update(SignalUpdate::IsLocked(true));
        assert!(result.is_some(), "expected Some(json) after first update");

        let json = result.unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("result should be valid JSON");

        assert_eq!(parsed["vin"], "VIN-001");
        assert_eq!(parsed["is_locked"], true);
        assert!(parsed["timestamp"].is_number(), "timestamp must be a number");
        assert!(
            parsed.get("latitude").is_none(),
            "latitude must be absent when never set"
        );
        assert!(
            parsed.get("longitude").is_none(),
            "longitude must be absent when never set"
        );
        assert!(
            parsed.get("parking_active").is_none(),
            "parking_active must be absent when never set"
        );
    }

    // ------------------------------------------------------------------
    // TS-04-8: Unset fields are omitted (Latitude-only case)
    // ------------------------------------------------------------------

    /// TS-04-8: After a single Latitude update, JSON contains latitude and omits
    /// all other optional signal fields.
    ///
    /// Validates [04-REQ-8.3]
    #[test]
    fn test_telemetry_omits_unset_fields() {
        let mut state = TelemetryState::new("VIN-001".to_string());
        let result = state.update(SignalUpdate::Latitude(48.1351));
        assert!(result.is_some(), "expected Some(json) after first update");

        let json = result.unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("result should be valid JSON");

        let lat = parsed["latitude"]
            .as_f64()
            .expect("latitude must be a number");
        assert!(
            (lat - 48.1351).abs() < 1e-6,
            "latitude value mismatch: {lat}"
        );
        assert!(
            parsed.get("is_locked").is_none(),
            "is_locked must be absent when never set"
        );
        assert!(
            parsed.get("longitude").is_none(),
            "longitude must be absent when never set"
        );
        assert!(
            parsed.get("parking_active").is_none(),
            "parking_active must be absent when never set"
        );
    }

    // ------------------------------------------------------------------
    // TS-04-9: All four signals included after four sequential updates
    // ------------------------------------------------------------------

    /// TS-04-9: After updates for all four signals, the final JSON includes every field.
    ///
    /// Validates [04-REQ-8.2]
    #[test]
    fn test_telemetry_all_fields_present() {
        let mut state = TelemetryState::new("VIN-001".to_string());
        state.update(SignalUpdate::IsLocked(true));
        state.update(SignalUpdate::Latitude(48.1351));
        state.update(SignalUpdate::Longitude(11.582));
        let result = state.update(SignalUpdate::ParkingActive(true));

        assert!(
            result.is_some(),
            "expected Some(json) after final update"
        );
        let json = result.unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("result should be valid JSON");

        assert_eq!(parsed["is_locked"], true);
        let lat = parsed["latitude"].as_f64().expect("latitude must be a number");
        assert!((lat - 48.1351).abs() < 1e-6, "latitude value mismatch");
        let lon = parsed["longitude"]
            .as_f64()
            .expect("longitude must be a number");
        assert!((lon - 11.582).abs() < 1e-6, "longitude value mismatch");
        assert_eq!(parsed["parking_active"], true);
    }

    // ------------------------------------------------------------------
    // TS-04-P5: Telemetry Completeness — property test
    // ------------------------------------------------------------------

    /// TS-04-P5: After any sequence of signal updates, the published JSON contains
    /// exactly the set of previously-updated fields, each with its latest value, and
    /// omits fields that have never been set.
    ///
    /// Validates [04-REQ-8.1], [04-REQ-8.2], [04-REQ-8.3]
    #[test]
    fn test_property_telemetry_completeness() {
        let mut state = TelemetryState::new("VIN".to_string());

        // After IsLocked: only is_locked present
        let json: serde_json::Value = serde_json::from_str(
            &state.update(SignalUpdate::IsLocked(false)).unwrap(),
        )
        .unwrap();
        assert!(json.get("is_locked").is_some(), "is_locked must be present");
        assert!(json.get("latitude").is_none(), "latitude must be absent");
        assert!(json.get("longitude").is_none(), "longitude must be absent");
        assert!(
            json.get("parking_active").is_none(),
            "parking_active must be absent"
        );

        // After Latitude: is_locked + latitude
        let json: serde_json::Value = serde_json::from_str(
            &state.update(SignalUpdate::Latitude(10.0)).unwrap(),
        )
        .unwrap();
        assert!(json.get("is_locked").is_some());
        assert!(json.get("latitude").is_some());
        assert!(json.get("longitude").is_none());
        assert!(json.get("parking_active").is_none());

        // After Longitude: is_locked + latitude + longitude
        let json: serde_json::Value = serde_json::from_str(
            &state.update(SignalUpdate::Longitude(20.0)).unwrap(),
        )
        .unwrap();
        assert!(json.get("is_locked").is_some());
        assert!(json.get("latitude").is_some());
        assert!(json.get("longitude").is_some());
        assert!(json.get("parking_active").is_none());

        // After ParkingActive: all four fields present
        let json: serde_json::Value = serde_json::from_str(
            &state.update(SignalUpdate::ParkingActive(true)).unwrap(),
        )
        .unwrap();
        assert!(json.get("is_locked").is_some());
        assert!(json.get("latitude").is_some());
        assert!(json.get("longitude").is_some());
        assert!(json.get("parking_active").is_some());

        // Latest value reflected: update IsLocked to new value
        let json: serde_json::Value = serde_json::from_str(
            &state.update(SignalUpdate::IsLocked(true)).unwrap(),
        )
        .unwrap();
        assert_eq!(json["is_locked"], true, "latest IsLocked value must be reflected");
    }
}
