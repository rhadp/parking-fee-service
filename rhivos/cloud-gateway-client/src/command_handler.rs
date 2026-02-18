//! Command handler for incoming MQTT lock/unlock commands.
//!
//! Receives [`CommandMessage`] payloads from the MQTT event loop, parses the
//! JSON, and writes the corresponding `Vehicle.Command.Door.Lock` value to
//! DATA_BROKER via the Kuksa client.
//!
//! Invalid JSON payloads are logged and discarded (03-REQ-3.E3).
//!
//! The most recent `command_id` is stored so that the [`result_forwarder`]
//! can correlate the `LockResult` response from DATA_BROKER back to the
//! originating command.
//!
//! # Requirements
//!
//! - 03-REQ-3.2: Lock command → write `Vehicle.Command.Door.Lock = true`.
//! - 03-REQ-3.3: Unlock command → write `Vehicle.Command.Door.Lock = false`.
//! - 03-REQ-3.E3: Invalid JSON → log error, discard message.

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::messages::{CommandMessage, CommandType};

/// Shared state for tracking the most recent command for response correlation.
///
/// The `command_id` and `command_type` from the most recently processed
/// command are stored here so the result forwarder can include them in the
/// `CommandResponse` published to MQTT.
#[derive(Debug, Clone)]
pub struct PendingCommand {
    pub command_id: String,
    pub command_type: CommandType,
}

/// Thread-safe container for the current pending command.
pub type PendingCommandState = Arc<Mutex<Option<PendingCommand>>>;

/// Create a new empty pending-command state.
pub fn new_pending_state() -> PendingCommandState {
    Arc::new(Mutex::new(None))
}

/// Trait abstracting DATA_BROKER write operations for testability.
///
/// In production this is implemented by the Kuksa client adapter. In tests
/// a mock implementation is used.
#[allow(async_fn_in_trait)]
pub trait DataBrokerWriter: Send + Sync {
    /// Write a boolean value to `Vehicle.Command.Door.Lock`.
    async fn set_door_lock(&self, lock: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Process a raw MQTT command payload.
///
/// 1. Parse JSON → [`CommandMessage`].
/// 2. Store the `command_id` in `pending` for response correlation.
/// 3. Write `Vehicle.Command.Door.Lock` to DATA_BROKER.
///
/// Returns `true` if the command was processed successfully, `false` if it
/// was discarded (invalid JSON or DATA_BROKER write failure).
pub async fn handle_command<W: DataBrokerWriter>(
    payload: &[u8],
    pending: &PendingCommandState,
    writer: &W,
) -> bool {
    // 1. Parse the command message JSON.
    let cmd: CommandMessage = match serde_json::from_slice(payload) {
        Ok(msg) => msg,
        Err(e) => {
            warn!(error = %e, "received invalid command JSON, discarding");
            return false;
        }
    };

    info!(
        command_id = %cmd.command_id,
        command_type = ?cmd.command_type,
        "received command"
    );

    // 2. Store the command_id for response correlation.
    {
        let mut guard = pending.lock().await;
        *guard = Some(PendingCommand {
            command_id: cmd.command_id.clone(),
            command_type: cmd.command_type,
        });
    }

    // 3. Write to DATA_BROKER.
    let lock_value = cmd.command_type == CommandType::Lock;
    if let Err(e) = writer.set_door_lock(lock_value).await {
        error!(
            command_id = %cmd.command_id,
            error = %e,
            "failed to write door lock command to DATA_BROKER"
        );
        return false;
    }

    info!(
        command_id = %cmd.command_id,
        lock_value,
        "wrote Vehicle.Command.Door.Lock to DATA_BROKER"
    );

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Mock DATA_BROKER writer that records calls.
    struct MockWriter {
        last_value: Mutex<Option<bool>>,
        should_fail: AtomicBool,
    }

    impl MockWriter {
        fn new() -> Self {
            Self {
                last_value: Mutex::new(None),
                should_fail: AtomicBool::new(false),
            }
        }

        fn fail_on_next(&self) {
            self.should_fail.store(true, Ordering::SeqCst);
        }

        async fn last_value(&self) -> Option<bool> {
            *self.last_value.lock().await
        }
    }

    impl DataBrokerWriter for MockWriter {
        async fn set_door_lock(&self, lock: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            if self.should_fail.load(Ordering::SeqCst) {
                return Err("mock write failure".into());
            }
            *self.last_value.lock().await = Some(lock);
            Ok(())
        }
    }

    #[tokio::test]
    async fn lock_command_writes_true() {
        let pending = new_pending_state();
        let writer = MockWriter::new();
        let payload = br#"{"command_id":"cmd-1","type":"lock","timestamp":1708300800}"#;

        let result = handle_command(payload, &pending, &writer).await;

        assert!(result, "command should succeed");
        assert_eq!(writer.last_value().await, Some(true));
        let cmd = pending.lock().await;
        assert_eq!(cmd.as_ref().unwrap().command_id, "cmd-1");
        assert_eq!(cmd.as_ref().unwrap().command_type, CommandType::Lock);
    }

    #[tokio::test]
    async fn unlock_command_writes_false() {
        let pending = new_pending_state();
        let writer = MockWriter::new();
        let payload = br#"{"command_id":"cmd-2","type":"unlock","timestamp":1708300801}"#;

        let result = handle_command(payload, &pending, &writer).await;

        assert!(result, "command should succeed");
        assert_eq!(writer.last_value().await, Some(false));
        let cmd = pending.lock().await;
        assert_eq!(cmd.as_ref().unwrap().command_id, "cmd-2");
        assert_eq!(cmd.as_ref().unwrap().command_type, CommandType::Unlock);
    }

    #[tokio::test]
    async fn invalid_json_discarded() {
        let pending = new_pending_state();
        let writer = MockWriter::new();
        let payload = b"not valid json";

        let result = handle_command(payload, &pending, &writer).await;

        assert!(!result, "invalid JSON should be discarded");
        assert_eq!(writer.last_value().await, None, "no write should occur");
        assert!(pending.lock().await.is_none(), "no pending command stored");
    }

    #[tokio::test]
    async fn missing_fields_discarded() {
        let pending = new_pending_state();
        let writer = MockWriter::new();
        // Missing the "type" field.
        let payload = br#"{"command_id":"cmd-3","timestamp":0}"#;

        let result = handle_command(payload, &pending, &writer).await;

        assert!(!result, "incomplete JSON should be discarded");
        assert_eq!(writer.last_value().await, None);
    }

    #[tokio::test]
    async fn databroker_failure_returns_false() {
        let pending = new_pending_state();
        let writer = MockWriter::new();
        writer.fail_on_next();
        let payload = br#"{"command_id":"cmd-4","type":"lock","timestamp":0}"#;

        let result = handle_command(payload, &pending, &writer).await;

        assert!(!result, "DATA_BROKER failure should return false");
        // The pending command is still stored (for potential retry).
        let cmd = pending.lock().await;
        assert!(cmd.is_some(), "pending command should be stored before write");
    }

    #[tokio::test]
    async fn subsequent_command_overwrites_pending() {
        let pending = new_pending_state();
        let writer = MockWriter::new();

        let payload1 = br#"{"command_id":"cmd-a","type":"lock","timestamp":0}"#;
        handle_command(payload1, &pending, &writer).await;

        let payload2 = br#"{"command_id":"cmd-b","type":"unlock","timestamp":1}"#;
        handle_command(payload2, &pending, &writer).await;

        let cmd = pending.lock().await;
        assert_eq!(cmd.as_ref().unwrap().command_id, "cmd-b");
        assert_eq!(cmd.as_ref().unwrap().command_type, CommandType::Unlock);
    }

    #[tokio::test]
    async fn invalid_command_type_discarded() {
        let pending = new_pending_state();
        let writer = MockWriter::new();
        let payload = br#"{"command_id":"cmd-5","type":"invalid","timestamp":0}"#;

        let result = handle_command(payload, &pending, &writer).await;

        assert!(!result, "unknown command type should be discarded");
        assert_eq!(writer.last_value().await, None);
    }
}
