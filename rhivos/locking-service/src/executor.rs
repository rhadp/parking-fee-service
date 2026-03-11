//! Lock/unlock execution and response writing.
//!
//! This module handles:
//! - Executing lock/unlock commands by writing state to DATA_BROKER.
//! - Writing command responses (success or failure) to DATA_BROKER.

use crate::command::CommandResponse;
use crate::databroker_client::{DataBrokerClient, SignalValue};
use tracing::error;

/// VSS signal path for the driver-side door lock state.
const LOCK_STATE_SIGNAL: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";

/// VSS signal path for the command response.
const RESPONSE_SIGNAL: &str = "Vehicle.Command.Door.Response";

/// Execute a lock or unlock command by writing the lock state to DATA_BROKER.
///
/// On `"lock"`: writes `IsLocked = true`.
/// On `"unlock"`: writes `IsLocked = false`.
///
/// Returns `Ok(())` on success, or an error message on failure.
pub async fn execute_lock_command(
    client: &mut DataBrokerClient,
    action: &str,
    command_id: &str,
) -> Result<(), String> {
    let lock_value = action == "lock";
    client
        .set_signal(LOCK_STATE_SIGNAL, SignalValue::Bool(lock_value))
        .await
        .map_err(|e| {
            error!(command_id = %command_id, "Failed to write lock state: {}", e);
            e.to_string()
        })
}

/// Write a command response JSON to the DATA_BROKER response signal.
pub async fn write_response(client: &mut DataBrokerClient, resp: &CommandResponse) {
    let json = resp.to_json();
    if let Err(e) = client
        .set_signal(RESPONSE_SIGNAL, SignalValue::String(json))
        .await
    {
        error!("Failed to write command response: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use crate::command::CommandResponse;

    #[test]
    fn test_success_response_has_correct_fields() {
        let resp = CommandResponse::success("test-id".to_string(), 1700000000);
        assert_eq!(resp.status, "success");
        assert!(resp.reason.is_none());
    }

    #[test]
    fn test_failure_response_has_correct_fields() {
        let resp =
            CommandResponse::failure("test-id".to_string(), "vehicle_moving".to_string(), 1700000000);
        assert_eq!(resp.status, "failed");
        assert_eq!(resp.reason, Some("vehicle_moving".to_string()));
    }
}
