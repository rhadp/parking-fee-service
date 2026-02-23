//! Main LOCKING_SERVICE logic.
//!
//! Subscribes to Vehicle.Command.Door.Lock signals from DATA_BROKER,
//! validates commands, checks safety constraints, executes lock/unlock
//! actions, and writes responses to Vehicle.Command.Door.Response.

use databroker_client::{DataValue, DatabrokerClient};
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};

use crate::command::{self, reason, CommandResponse, LockAction, ParseResult};
use crate::safety::SafetyChecker;

/// VSS signal paths used by the locking service.
pub mod signals {
    /// Command signal: incoming lock/unlock commands (JSON string).
    pub const COMMAND: &str = "Vehicle.Command.Door.Lock";
    /// Response signal: command execution results (JSON string).
    pub const RESPONSE: &str = "Vehicle.Command.Door.Response";
    /// Lock state signal: current door lock state (bool).
    pub const IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
}

/// Run the LOCKING_SERVICE main loop.
///
/// Connects to DATA_BROKER, subscribes to command signals, and processes
/// each command as it arrives.
pub async fn run(endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!(endpoint = %endpoint, "connecting to DATA_BROKER");

    let client = DatabrokerClient::connect(endpoint).await?;
    let safety = SafetyChecker::new(client.clone());

    info!("subscribing to {}", signals::COMMAND);
    let mut stream = client.subscribe(&[signals::COMMAND]).await?;

    info!("locking-service started, waiting for commands");

    while let Some(result) = stream.next().await {
        match result {
            Ok(updates) => {
                for update in updates {
                    if update.path != signals::COMMAND {
                        continue;
                    }

                    let payload = match &update.value {
                        Some(DataValue::String(s)) => s.clone(),
                        Some(other) => {
                            warn!(value = ?other, "non-string value on command signal, ignoring");
                            continue;
                        }
                        None => {
                            debug!("empty value on command signal, ignoring");
                            continue;
                        }
                    };

                    debug!(payload = %payload, "received command signal");

                    let response = process_command(&payload, &client, &safety).await;

                    // Write response to Vehicle.Command.Door.Response
                    let response_json = response.to_json();
                    debug!(response = %response_json, "writing command response");

                    if let Err(e) = client
                        .set_value(signals::RESPONSE, DataValue::String(response_json))
                        .await
                    {
                        error!(error = %e, "failed to write command response");
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "subscription error");
                break;
            }
        }
    }

    warn!("command subscription ended");
    Ok(())
}

/// Process a single command payload and return the appropriate response.
async fn process_command(
    payload: &str,
    client: &DatabrokerClient,
    safety: &SafetyChecker,
) -> CommandResponse {
    // Step 1: Parse the command
    let cmd = match command::parse_command(payload) {
        ParseResult::Ok(cmd) => cmd,
        ParseResult::InvalidPayload => {
            info!("rejected command: invalid JSON payload");
            return CommandResponse::failed_no_id(reason::INVALID_PAYLOAD);
        }
        ParseResult::MissingFields => {
            info!("rejected command: missing required fields");
            return CommandResponse::failed_no_id(reason::MISSING_FIELDS);
        }
        ParseResult::UnknownAction { command_id } => {
            let id = command_id.as_deref().unwrap_or("");
            info!(command_id = %id, "rejected command: unknown action");
            return CommandResponse::failed(id, reason::UNKNOWN_ACTION);
        }
    };

    info!(
        command_id = %cmd.command_id,
        action = ?cmd.action,
        "processing command"
    );

    // Step 2: Check safety constraints
    let constraint_result = match cmd.action {
        LockAction::Lock => safety.check_lock_constraints().await,
        LockAction::Unlock => safety.check_unlock_constraints().await,
    };

    if let Err(reason) = constraint_result {
        info!(
            command_id = %cmd.command_id,
            reason = %reason,
            "command rejected by safety check"
        );
        return CommandResponse::failed(&cmd.command_id, &reason);
    }

    // Step 3: Execute the lock/unlock action
    let lock_value = matches!(cmd.action, LockAction::Lock);

    if let Err(e) = client
        .set_value(signals::IS_LOCKED, DataValue::Bool(lock_value))
        .await
    {
        error!(
            command_id = %cmd.command_id,
            error = %e,
            "failed to write lock state"
        );
        return CommandResponse::failed(&cmd.command_id, "write_failed");
    }

    info!(
        command_id = %cmd.command_id,
        locked = lock_value,
        "command executed successfully"
    );

    CommandResponse::success(&cmd.command_id)
}
