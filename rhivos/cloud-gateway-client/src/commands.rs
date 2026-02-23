//! Command validation and DATA_BROKER writing.
//!
//! Validates incoming MQTT command messages and writes validated commands
//! to Vehicle.Command.Door.Lock in DATA_BROKER. Also subscribes to
//! Vehicle.Command.Door.Response and relays responses back to MQTT.

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// A lock/unlock command received from the cloud gateway.
///
/// Must contain at minimum `command_id` and `action`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockCommand {
    /// Unique identifier for this command.
    pub command_id: String,
    /// The requested action: "lock" or "unlock".
    pub action: String,
    /// The doors to act on (e.g., `["driver"]`).
    #[serde(default)]
    pub doors: Vec<String>,
    /// The source of the command (e.g., `"companion_app"`).
    #[serde(default)]
    pub source: String,
    /// Vehicle identification number.
    #[serde(default)]
    pub vin: String,
    /// Unix timestamp of when the command was issued.
    #[serde(default)]
    pub timestamp: i64,
}

/// Validate an MQTT command message.
///
/// Checks that the payload is valid JSON containing a `command_id` and
/// `action` field. Returns the validated JSON string (re-serialized from
/// the parsed structure) or `None` if invalid.
///
/// # Arguments
///
/// * `payload` - Raw MQTT message payload bytes.
///
/// # Returns
///
/// `Some(json_string)` if the command is valid, `None` otherwise.
pub fn validate_command(payload: &[u8]) -> Option<String> {
    // Parse as UTF-8
    let text = match std::str::from_utf8(payload) {
        Ok(s) => s,
        Err(e) => {
            warn!(error = %e, "MQTT command payload is not valid UTF-8");
            return None;
        }
    };

    // Parse as JSON
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "MQTT command payload is not valid JSON");
            return None;
        }
    };

    // Validate required fields
    let obj = match value.as_object() {
        Some(o) => o,
        None => {
            warn!("MQTT command payload is not a JSON object");
            return None;
        }
    };

    if !obj.contains_key("command_id") {
        warn!("MQTT command missing required field: command_id");
        return None;
    }

    if !obj.contains_key("action") {
        warn!("MQTT command missing required field: action");
        return None;
    }

    // Verify action is a recognized value
    if let Some(action) = obj.get("action").and_then(|v| v.as_str()) {
        if action != "lock" && action != "unlock" {
            warn!(action = %action, "MQTT command has unknown action");
            return None;
        }
    } else {
        warn!("MQTT command action is not a string");
        return None;
    }

    // Parse the full command to verify structure
    match serde_json::from_str::<LockCommand>(text) {
        Ok(cmd) => {
            info!(
                command_id = %cmd.command_id,
                action = %cmd.action,
                "validated MQTT command"
            );
            // Return the original text to preserve the exact payload
            Some(text.to_string())
        }
        Err(e) => {
            warn!(error = %e, "MQTT command failed full validation");
            None
        }
    }
}

/// Build a telemetry message JSON string from signal updates.
///
/// The telemetry message format follows the spec:
/// ```json
/// {
///     "signals": [
///         {"path": "<vss_path>", "value": <value>, "timestamp": <unix_ts>}
///     ]
/// }
/// ```
pub fn build_telemetry_message(signals: &[TelemetrySignal]) -> String {
    let msg = TelemetryMessage { signals };
    serde_json::to_string(&msg).expect("telemetry message should serialize")
}

/// A single signal in a telemetry message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySignal {
    /// The VSS signal path.
    pub path: String,
    /// The signal value.
    pub value: serde_json::Value,
    /// Unix timestamp.
    pub timestamp: i64,
}

/// The top-level telemetry message structure.
#[derive(Debug, Serialize)]
struct TelemetryMessage<'a> {
    signals: &'a [TelemetrySignal],
}

/// Get the current Unix timestamp in seconds.
pub fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_lock_command() {
        let payload = br#"{"command_id":"test-1","action":"lock","doors":["driver"],"source":"companion_app","vin":"VIN12345","timestamp":1700000000}"#;
        let result = validate_command(payload);
        assert!(result.is_some());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["command_id"], "test-1");
        assert_eq!(json["action"], "lock");
    }

    #[test]
    fn test_validate_valid_unlock_command() {
        let payload = br#"{"command_id":"test-2","action":"unlock","doors":["driver"],"source":"test","vin":"VIN12345","timestamp":1700000000}"#;
        let result = validate_command(payload);
        assert!(result.is_some());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["command_id"], "test-2");
        assert_eq!(json["action"], "unlock");
    }

    #[test]
    fn test_validate_invalid_json() {
        let payload = b"not valid json {{{";
        assert!(validate_command(payload).is_none());
    }

    #[test]
    fn test_validate_missing_command_id() {
        let payload = br#"{"action":"lock","doors":["driver"]}"#;
        assert!(validate_command(payload).is_none());
    }

    #[test]
    fn test_validate_missing_action() {
        let payload = br#"{"command_id":"test-3","doors":["driver"]}"#;
        assert!(validate_command(payload).is_none());
    }

    #[test]
    fn test_validate_unknown_action() {
        let payload = br#"{"command_id":"test-4","action":"toggle","doors":["driver"]}"#;
        assert!(validate_command(payload).is_none());
    }

    #[test]
    fn test_validate_non_object_json() {
        let payload = br#""just a string""#;
        assert!(validate_command(payload).is_none());
    }

    #[test]
    fn test_validate_non_utf8() {
        let payload: &[u8] = &[0xFF, 0xFE, 0xFD];
        assert!(validate_command(payload).is_none());
    }

    #[test]
    fn test_build_telemetry_message() {
        let signals = vec![TelemetrySignal {
            path: "Vehicle.Speed".to_string(),
            value: serde_json::json!(42.0),
            timestamp: 1700000000,
        }];
        let msg = build_telemetry_message(&signals);
        let parsed: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert!(parsed["signals"].is_array());
        assert_eq!(parsed["signals"][0]["path"], "Vehicle.Speed");
        assert_eq!(parsed["signals"][0]["value"], 42.0);
        assert_eq!(parsed["signals"][0]["timestamp"], 1700000000);
    }

    #[test]
    fn test_build_telemetry_message_multiple_signals() {
        let signals = vec![
            TelemetrySignal {
                path: "Vehicle.Speed".to_string(),
                value: serde_json::json!(60.0),
                timestamp: 1700000000,
            },
            TelemetrySignal {
                path: "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked".to_string(),
                value: serde_json::json!(true),
                timestamp: 1700000001,
            },
        ];
        let msg = build_telemetry_message(&signals);
        let parsed: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(parsed["signals"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_validate_minimal_command() {
        let payload = br#"{"command_id":"min","action":"lock"}"#;
        let result = validate_command(payload);
        assert!(result.is_some());
    }

    #[test]
    fn test_validate_preserves_original_payload() {
        let original = r#"{"command_id":"preserve","action":"lock","doors":["driver"],"source":"companion_app","vin":"VIN12345","timestamp":1700000000}"#;
        let result = validate_command(original.as_bytes());
        assert!(result.is_some());
        assert_eq!(result.unwrap(), original);
    }
}
