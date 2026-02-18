//! Result forwarder — subscribes to `Vehicle.Command.Door.LockResult` on
//! DATA_BROKER and publishes a [`CommandResponse`] to MQTT.
//!
//! When the LOCKING_SERVICE processes a lock/unlock command it writes the
//! result (e.g. `"SUCCESS"`, `"REJECTED_SPEED"`) to the `LockResult` signal
//! in DATA_BROKER. This module subscribes to that signal and, on each
//! change, publishes a `CommandResponse` MQTT message that includes the
//! `command_id` from the most recently received command.
//!
//! # Requirements
//!
//! - 03-REQ-3.4: Subscribe to `LockResult` on DATA_BROKER. On result change,
//!   publish `CommandResponse` to `vehicles/{vin}/command_responses` (QoS 2).

use tracing::{error, info, warn};

use crate::command_handler::PendingCommandState;
use crate::messages::{CommandResponse, CommandResult};
use crate::mqtt::MqttClient;

/// Trait abstracting DATA_BROKER subscription to `LockResult` for testability.
#[allow(async_fn_in_trait)]
pub trait LockResultSubscriber: Send + Sync {
    /// Subscribe to `Vehicle.Command.Door.LockResult` signal changes.
    ///
    /// Returns a stream of result strings (e.g. `"SUCCESS"`, `"REJECTED_SPEED"`).
    async fn subscribe_lock_result(
        &self,
    ) -> Result<
        Box<dyn tokio_stream::Stream<Item = Result<String, Box<dyn std::error::Error + Send + Sync>>> + Send + Unpin>,
        Box<dyn std::error::Error + Send + Sync>,
    >;
}

/// Run the result-forwarding loop.
///
/// Subscribes to `LockResult` changes on DATA_BROKER and publishes a
/// `CommandResponse` to MQTT for each change. The `command_id` is taken
/// from the pending command state maintained by the command handler.
///
/// This function runs until the subscription stream ends or an
/// unrecoverable error occurs.
pub async fn run_result_forwarder<S: LockResultSubscriber>(
    subscriber: &S,
    mqtt: &MqttClient,
    pending: &PendingCommandState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio_stream::StreamExt;

    info!("starting result forwarder, subscribing to LockResult");

    let mut stream = subscriber.subscribe_lock_result().await?;

    while let Some(result) = stream.next().await {
        let result_str = match result {
            Ok(s) => s,
            Err(e) => {
                error!(error = %e, "error in LockResult subscription stream");
                continue;
            }
        };

        info!(result = %result_str, "received LockResult from DATA_BROKER");

        // Parse the result string into our enum.
        let command_result = match parse_lock_result(&result_str) {
            Some(r) => r,
            None => {
                warn!(result = %result_str, "unknown LockResult value, skipping");
                continue;
            }
        };

        // Get the pending command for correlation.
        let pending_cmd = {
            let guard = pending.lock().await;
            guard.clone()
        };

        let pending_cmd = match pending_cmd {
            Some(cmd) => cmd,
            None => {
                warn!("received LockResult but no pending command, skipping");
                continue;
            }
        };

        // Construct and publish the command response.
        let response = CommandResponse {
            command_id: pending_cmd.command_id.clone(),
            command_type: pending_cmd.command_type,
            result: command_result,
            timestamp: chrono_timestamp(),
        };

        let payload = match serde_json::to_vec(&response) {
            Ok(p) => p,
            Err(e) => {
                error!(error = %e, "failed to serialize CommandResponse");
                continue;
            }
        };

        if let Err(e) = mqtt.publish_command_response(&payload).await {
            error!(
                command_id = %pending_cmd.command_id,
                error = %e,
                "failed to publish command response to MQTT"
            );
        } else {
            info!(
                command_id = %pending_cmd.command_id,
                result = %result_str,
                "published command response to MQTT"
            );
        }
    }

    warn!("LockResult subscription stream ended");
    Ok(())
}

/// Parse a `LockResult` string from DATA_BROKER into a [`CommandResult`].
fn parse_lock_result(s: &str) -> Option<CommandResult> {
    match s {
        "SUCCESS" => Some(CommandResult::SUCCESS),
        "REJECTED_SPEED" => Some(CommandResult::REJECTED_SPEED),
        "REJECTED_DOOR_OPEN" => Some(CommandResult::REJECTED_DOOR_OPEN),
        _ => None,
    }
}

/// Returns the current Unix timestamp in seconds.
fn chrono_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_handler::{new_pending_state, PendingCommand};
    use crate::messages::CommandType;

    #[test]
    fn parse_lock_result_success() {
        assert_eq!(parse_lock_result("SUCCESS"), Some(CommandResult::SUCCESS));
    }

    #[test]
    fn parse_lock_result_rejected_speed() {
        assert_eq!(
            parse_lock_result("REJECTED_SPEED"),
            Some(CommandResult::REJECTED_SPEED)
        );
    }

    #[test]
    fn parse_lock_result_rejected_door_open() {
        assert_eq!(
            parse_lock_result("REJECTED_DOOR_OPEN"),
            Some(CommandResult::REJECTED_DOOR_OPEN)
        );
    }

    #[test]
    fn parse_lock_result_unknown() {
        assert_eq!(parse_lock_result("UNKNOWN"), None);
        assert_eq!(parse_lock_result(""), None);
    }

    #[test]
    fn chrono_timestamp_is_reasonable() {
        let ts = chrono_timestamp();
        assert!(ts > 1_577_836_800, "timestamp too small: {ts}");
        assert!(ts < 4_102_444_800, "timestamp too large: {ts}");
    }

    // ── Integration-style tests with mock subscriber and MQTT ───────────

    /// A mock LockResult subscriber that yields a predefined sequence of results.
    ///
    /// Stores results as `Option<String>` — `Some(s)` for success, `None` for
    /// error — so the inner vec is `Clone`.
    struct MockLockResultSubscriber {
        results: Vec<Option<String>>,
    }

    impl MockLockResultSubscriber {
        fn new(results: Vec<&str>) -> Self {
            Self {
                results: results.into_iter().map(|s| Some(s.to_string())).collect(),
            }
        }

        fn with_error(mut self) -> Self {
            self.results.push(None);
            self
        }
    }

    impl LockResultSubscriber for MockLockResultSubscriber {
        async fn subscribe_lock_result(
            &self,
        ) -> Result<
            Box<dyn tokio_stream::Stream<Item = Result<String, Box<dyn std::error::Error + Send + Sync>>> + Send + Unpin>,
            Box<dyn std::error::Error + Send + Sync>,
        > {
            let items: Vec<Result<String, Box<dyn std::error::Error + Send + Sync>>> = self
                .results
                .iter()
                .map(|opt| match opt {
                    Some(s) => Ok(s.clone()),
                    None => Err(Box::<dyn std::error::Error + Send + Sync>::from(
                        "mock subscription error",
                    )),
                })
                .collect();
            let stream = tokio_stream::iter(items);
            Ok(Box::new(stream))
        }
    }

    // We need a mock MQTT client for testing.  Since `MqttClient` wraps
    // `rumqttc::AsyncClient` which requires a real connection, we test the
    // logic in `parse_lock_result` and pending-state correlation here.
    // Full MQTT publish tests are in the integration test suite.

    #[tokio::test]
    async fn pending_command_correlation() {
        let pending = new_pending_state();

        // Set a pending command.
        {
            let mut guard = pending.lock().await;
            *guard = Some(PendingCommand {
                command_id: "test-cmd-1".to_string(),
                command_type: CommandType::Lock,
            });
        }

        // Verify we can read it back.
        let cmd = pending.lock().await;
        let cmd = cmd.as_ref().unwrap();
        assert_eq!(cmd.command_id, "test-cmd-1");
        assert_eq!(cmd.command_type, CommandType::Lock);
    }

    #[tokio::test]
    async fn command_response_serialization() {
        let response = CommandResponse {
            command_id: "test-cmd-1".to_string(),
            command_type: CommandType::Lock,
            result: CommandResult::SUCCESS,
            timestamp: 1708300801,
        };

        let payload = serde_json::to_vec(&response).unwrap();
        let parsed: CommandResponse = serde_json::from_slice(&payload).unwrap();

        assert_eq!(parsed.command_id, "test-cmd-1");
        assert_eq!(parsed.command_type, CommandType::Lock);
        assert_eq!(parsed.result, CommandResult::SUCCESS);
        assert_eq!(parsed.timestamp, 1708300801);
    }

    #[tokio::test]
    async fn command_response_rejected_speed() {
        let response = CommandResponse {
            command_id: "test-cmd-2".to_string(),
            command_type: CommandType::Lock,
            result: CommandResult::REJECTED_SPEED,
            timestamp: 1708300802,
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["result"], "REJECTED_SPEED");
        assert_eq!(json["type"], "lock");
    }

    #[test]
    fn mock_subscriber_creates_stream() {
        let subscriber = MockLockResultSubscriber::new(vec!["SUCCESS", "REJECTED_SPEED"]);
        assert_eq!(subscriber.results.len(), 2);
    }

    #[test]
    fn mock_subscriber_with_error() {
        let subscriber = MockLockResultSubscriber::new(vec!["SUCCESS"]).with_error();
        assert_eq!(subscriber.results.len(), 2);
    }
}
