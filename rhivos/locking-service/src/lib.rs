pub mod command;
pub mod config;
pub mod databroker_client;
pub mod executor;
pub mod safety;

use command::{Command, CommandResponse, ValidationError};
use databroker_client::{DataBrokerClient, SignalValue};
use tracing::info;

/// Command signal path in DATA_BROKER.
pub const COMMAND_SIGNAL: &str = "Vehicle.Command.Door.Lock";

/// Process a single command JSON string from the subscription stream.
///
/// Follows the command processing flow:
/// 1. Parse JSON -> reject malformed with "invalid_command"
/// 2. Validate fields -> reject missing fields with "invalid_command"
/// 3. Validate action -> reject unknown action with "invalid_action"
/// 4. Safety checks -> reject constraint violations with specific reason
/// 5. Execute lock/unlock -> write state and success response
pub async fn process_command(json_str: &str, client: &mut DataBrokerClient) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Step 1 & 2: Parse and validate the command JSON.
    let cmd = match Command::from_json(json_str) {
        Ok(cmd) => cmd,
        Err(ValidationError::MalformedJson(e)) => {
            tracing::warn!("Malformed command JSON: {}", e);
            let resp =
                CommandResponse::failure("unknown".to_string(), "invalid_command".to_string(), now);
            executor::write_response(client, &resp).await;
            return;
        }
        Err(ValidationError::MissingField(field)) => {
            tracing::warn!("Command missing required field: {}", field);
            let resp =
                CommandResponse::failure("unknown".to_string(), "invalid_command".to_string(), now);
            executor::write_response(client, &resp).await;
            return;
        }
        Err(ValidationError::InvalidAction(action)) => {
            tracing::warn!("Invalid action: {}", action);
            let resp =
                CommandResponse::failure("unknown".to_string(), "invalid_action".to_string(), now);
            executor::write_response(client, &resp).await;
            return;
        }
    };

    let command_id = cmd.command_id.clone();

    // Step 3: Validate action is "lock" or "unlock".
    if let Err(ValidationError::InvalidAction(_)) = cmd.validate_action() {
        tracing::warn!(command_id = %command_id, "Invalid action: {}", cmd.action);
        let resp = CommandResponse::failure(command_id, "invalid_action".to_string(), now);
        executor::write_response(client, &resp).await;
        return;
    }

    // Step 4: Safety constraint checks.
    let speed = match client.get_signal("Vehicle.Speed").await {
        Ok(Some(SignalValue::Float(f))) => Some(f as f64),
        Ok(Some(SignalValue::Double(d))) => Some(d),
        _ => None,
    };

    let door_open = match client
        .get_signal("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen")
        .await
    {
        Ok(Some(SignalValue::Bool(b))) => Some(b),
        _ => None,
    };

    if let Err(reason) = safety::check_safety_constraints(speed, door_open) {
        info!(command_id = %command_id, reason = %reason, "Safety constraint violated");
        let resp = CommandResponse::failure(command_id, reason, now);
        executor::write_response(client, &resp).await;
        return;
    }

    // Step 5: Execute lock/unlock and write success response.
    let _ = executor::execute_lock_command(client, &cmd.action, &command_id).await;

    let resp = CommandResponse::success(command_id, now);
    executor::write_response(client, &resp).await;
}
