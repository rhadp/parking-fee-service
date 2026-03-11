//! Inbound command processing pipeline: NATS -> validate -> DATA_BROKER.
//!
//! Reads messages from the NATS command subscription, deserializes and validates
//! each command, and writes valid commands to `Vehicle.Command.Door.Lock` on DATA_BROKER.

use futures::StreamExt;
use tracing::{error, info, warn};

use crate::command::Command;
use crate::databroker_client::{DataBrokerClient, SignalValue};

/// The VSS signal path for door lock commands.
const DOOR_LOCK_SIGNAL: &str = "Vehicle.Command.Door.Lock";

/// Run the command processing loop.
///
/// Reads messages from the NATS subscription, validates each command, and writes
/// valid commands to DATA_BROKER. Invalid commands are logged and discarded.
/// If DATA_BROKER is unreachable, commands are logged and discarded.
pub async fn run(
    mut commands_sub: async_nats::Subscriber,
    mut databroker: DataBrokerClient,
    databroker_uds_path: String,
) {
    info!("Command processor started");

    while let Some(msg) = commands_sub.next().await {
        let payload = match std::str::from_utf8(&msg.payload) {
            Ok(s) => s,
            Err(e) => {
                warn!("Command payload is not valid UTF-8: {}", e);
                continue;
            }
        };

        // Parse and validate the command
        let cmd = match Command::from_json(payload) {
            Ok(cmd) => cmd,
            Err(e) => {
                warn!("Command validation failed: {}", e);
                continue;
            }
        };

        if let Err(e) = cmd.validate() {
            warn!("Command rejected: {}", e);
            continue;
        }

        info!(
            "Processing valid command: command_id={}, action={}",
            cmd.command_id, cmd.action
        );

        // Write command JSON to DATA_BROKER
        match databroker
            .set_signal(DOOR_LOCK_SIGNAL, SignalValue::String(payload.to_string()))
            .await
        {
            Ok(()) => {
                info!(
                    "Command {} written to DATA_BROKER at {}",
                    cmd.command_id, DOOR_LOCK_SIGNAL
                );
            }
            Err(e) => {
                error!(
                    "Failed to write command {} to DATA_BROKER: {}. Command discarded.",
                    cmd.command_id, e
                );
                // Attempt reconnection for the next command
                if let Err(re) = databroker.reconnect(&databroker_uds_path).await {
                    error!("DATA_BROKER reconnection failed: {}", re);
                }
            }
        }
    }

    info!("Command processor stopped (NATS subscription ended)");
}
