use tracing::{error, info};

use crate::command::CommandResponse;
use crate::databroker_client::DatabrokerClient;

/// VSS signal path for door lock state.
const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";

/// VSS signal path for command responses.
const SIGNAL_RESPONSE: &str = "Vehicle.Command.Door.Response";

/// Execute a lock or unlock action by writing the new state to DATA_BROKER.
///
/// - On `"lock"`: writes `IsLocked = true`
/// - On `"unlock"`: writes `IsLocked = false`
///
/// Returns `Ok(())` if the state write succeeds, or an error string on failure.
pub async fn execute_lock_action(
    client: &mut DatabrokerClient,
    action: &str,
) -> Result<(), String> {
    let lock_value = action == "lock";

    client
        .set_signal_bool(SIGNAL_IS_LOCKED, lock_value)
        .await
        .map_err(|e| {
            error!(
                signal = SIGNAL_IS_LOCKED,
                value = lock_value,
                error = %e,
                "Failed to write lock state to DATA_BROKER"
            );
            format!("Failed to write lock state: {e}")
        })?;

    info!(
        signal = SIGNAL_IS_LOCKED,
        value = lock_value,
        "Lock state updated"
    );

    Ok(())
}

/// Write a command response to DATA_BROKER.
///
/// Serializes the `CommandResponse` to JSON and writes it to
/// `Vehicle.Command.Door.Response`.
pub async fn write_response(
    client: &mut DatabrokerClient,
    response: &CommandResponse,
) {
    let json = response.to_json();

    if let Err(e) = client.set_signal_string(SIGNAL_RESPONSE, &json).await {
        error!(
            signal = SIGNAL_RESPONSE,
            error = %e,
            "Failed to write command response to DATA_BROKER"
        );
    } else {
        info!(
            signal = SIGNAL_RESPONSE,
            command_id = %response.command_id,
            status = %response.status,
            "Command response written"
        );
    }
}
