//! Inbound command pipeline: NATS -> validate -> DATA_BROKER.
//!
//! Reads messages from the NATS command subscription, validates each one,
//! and writes valid commands to the `Vehicle.Command.Door.Lock` signal
//! on DATA_BROKER via gRPC.

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::command::Command;
use crate::databroker_client::DatabrokerClient;

/// VSS signal path for incoming lock/unlock commands.
const SIGNAL_COMMAND_LOCK: &str = "Vehicle.Command.Door.Lock";

/// Run the command processing loop.
///
/// Reads messages from the NATS subscription stream, validates each command,
/// and writes valid commands to DATA_BROKER. Invalid commands are logged and
/// discarded. If DATA_BROKER is unreachable, commands are logged and discarded.
pub async fn run(
    mut subscription: async_nats::Subscriber,
    databroker: Arc<Mutex<DatabrokerClient>>,
) {
    use futures::StreamExt;

    info!("Command processor started");

    while let Some(message) = subscription.next().await {
        let payload = &message.payload;

        // Parse and validate the command
        let cmd = match Command::from_json(payload) {
            Ok(cmd) => cmd,
            Err(e) => {
                warn!(
                    error = %e,
                    payload_len = payload.len(),
                    "Discarding invalid command"
                );
                continue;
            }
        };

        info!(
            command_id = %cmd.command_id,
            action = %cmd.action,
            vin = %cmd.vin,
            "Received valid command, writing to DATA_BROKER"
        );

        // Serialize the command back to JSON for DATA_BROKER
        let json = match serde_json::to_string(&cmd) {
            Ok(json) => json,
            Err(e) => {
                error!(
                    command_id = %cmd.command_id,
                    error = %e,
                    "Failed to serialize command for DATA_BROKER"
                );
                continue;
            }
        };

        // Write the command to DATA_BROKER
        let mut db = databroker.lock().await;
        match db.set_signal_string(SIGNAL_COMMAND_LOCK, &json).await {
            Ok(()) => {
                info!(
                    command_id = %cmd.command_id,
                    signal = SIGNAL_COMMAND_LOCK,
                    "Command written to DATA_BROKER"
                );
            }
            Err(e) => {
                error!(
                    command_id = %cmd.command_id,
                    signal = SIGNAL_COMMAND_LOCK,
                    error = %e,
                    "Failed to write command to DATA_BROKER, discarding"
                );
            }
        }
    }

    warn!("Command processor subscription stream ended");
}
