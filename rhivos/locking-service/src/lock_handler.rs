//! Lock command processing loop.
//!
//! This module implements the core command handler that subscribes to lock/unlock
//! commands from the Kuksa Databroker, validates them against safety rules, and
//! writes the results back to the databroker.
//!
//! # Architecture
//!
//! The handler uses a [`DataBroker`] trait to abstract the Kuksa client, enabling
//! unit testing with mock implementations. The real [`KuksaDataBroker`] adapter
//! wraps [`parking_proto::kuksa_client::KuksaClient`].
//!
//! # Requirements
//!
//! - 02-REQ-2.2: Process each lock command signal change within the subscription stream
//! - 02-REQ-3.E1: Treat missing speed as 0.0 (safe default)
//! - 02-REQ-3.E2: Treat missing door signal as closed (safe default)
//! - 02-REQ-4.1: Write IsLocked when command passes safety validation
//! - 02-REQ-4.2: Do NOT modify IsLocked on rejection
//! - 02-REQ-4.E1: Log error if writing IsLocked fails, still report result
//! - 02-REQ-5.1: Write LockResult = "SUCCESS" on success
//! - 02-REQ-5.2: Write LockResult = "REJECTED_SPEED" on speed rejection
//! - 02-REQ-5.3: Write LockResult = "REJECTED_DOOR_OPEN" on door-ajar rejection
//! - 02-REQ-5.4: Write exactly one LockResult for every lock command processed
//! - 02-REQ-5.E1: Log error if writing LockResult fails (best-effort)

use parking_proto::kuksa_client::KuksaClient;
use parking_proto::signals;
use tracing::{error, info};

use crate::config::Config;
use crate::safety::{self, LockResult};

/// Trait abstracting Kuksa Databroker operations for testability.
///
/// The real implementation ([`KuksaDataBroker`]) wraps [`KuksaClient`].
/// Tests inject a mock implementation.
#[allow(async_fn_in_trait)]
pub trait DataBroker: Send + Sync {
    /// Read the current vehicle speed (km/h). Returns `None` if not set.
    async fn get_speed(&self) -> Result<Option<f32>, BrokerError>;

    /// Read whether the driver-side door is open. Returns `None` if not set.
    async fn get_door_is_open(&self) -> Result<Option<bool>, BrokerError>;

    /// Write the driver-side door locked state.
    async fn set_is_locked(&self, locked: bool) -> Result<(), BrokerError>;

    /// Write the lock command result string.
    async fn set_lock_result(&self, result: &str) -> Result<(), BrokerError>;
}

/// Errors from [`DataBroker`] operations.
#[derive(Debug, thiserror::Error)]
pub enum BrokerError {
    /// A Kuksa client error occurred.
    #[error("databroker error: {0}")]
    Client(String),
}

impl From<parking_proto::kuksa_client::Error> for BrokerError {
    fn from(err: parking_proto::kuksa_client::Error) -> Self {
        BrokerError::Client(err.to_string())
    }
}

/// Real [`DataBroker`] implementation wrapping [`KuksaClient`].
#[derive(Debug, Clone)]
pub struct KuksaDataBroker {
    client: KuksaClient,
}

impl KuksaDataBroker {
    /// Create a new adapter from a connected [`KuksaClient`].
    pub fn new(client: KuksaClient) -> Self {
        Self { client }
    }
}

impl DataBroker for KuksaDataBroker {
    async fn get_speed(&self) -> Result<Option<f32>, BrokerError> {
        Ok(self.client.get_f32(signals::SPEED).await?)
    }

    async fn get_door_is_open(&self) -> Result<Option<bool>, BrokerError> {
        Ok(self.client.get_bool(signals::DOOR_IS_OPEN).await?)
    }

    async fn set_is_locked(&self, locked: bool) -> Result<(), BrokerError> {
        Ok(self.client.set_bool(signals::DOOR_IS_LOCKED, locked).await?)
    }

    async fn set_lock_result(&self, result: &str) -> Result<(), BrokerError> {
        Ok(self
            .client
            .set_string(signals::LOCK_RESULT, result)
            .await?)
    }
}

/// Process a single lock command against the current vehicle state.
///
/// This is the core logic invoked for each signal change on
/// `Vehicle.Command.Door.Lock`. It:
///
/// 1. Reads current speed and door state from the databroker.
/// 2. Validates the command using [`safety::validate_lock`].
/// 3. If valid, writes the new lock state.
/// 4. Always writes the result string (exactly one per command).
///
/// # Safe Defaults
///
/// - Missing speed signal is treated as 0.0 km/h (02-REQ-3.E1).
/// - Missing door signal is treated as closed (02-REQ-3.E2).
pub async fn process_lock_command<B: DataBroker>(
    broker: &B,
    command_is_lock: bool,
    max_speed_kmh: f32,
) {
    let action = if command_is_lock { "lock" } else { "unlock" };
    info!(command = action, "processing lock command");

    // Read current vehicle state with safe defaults.
    let speed = match broker.get_speed().await {
        Ok(Some(s)) => {
            info!(speed_kmh = s, "read vehicle speed");
            s
        }
        Ok(None) => {
            info!("vehicle speed not set, using safe default 0.0");
            0.0
        }
        Err(e) => {
            error!(error = %e, "failed to read speed, using safe default 0.0");
            0.0
        }
    };

    let door_is_open = match broker.get_door_is_open().await {
        Ok(Some(d)) => {
            info!(door_is_open = d, "read door state");
            d
        }
        Ok(None) => {
            info!("door state not set, using safe default: closed");
            false
        }
        Err(e) => {
            error!(error = %e, "failed to read door state, using safe default: closed");
            false
        }
    };

    // Validate the command.
    let result = safety::validate_lock(command_is_lock, speed, door_is_open, max_speed_kmh);
    info!(result = %result, command = action, "safety validation complete");

    // Execute if valid (02-REQ-4.1), skip if rejected (02-REQ-4.2).
    if result == LockResult::Success {
        if let Err(e) = broker.set_is_locked(command_is_lock).await {
            // 02-REQ-4.E1: log error but continue to report result.
            error!(error = %e, "failed to write IsLocked");
        } else {
            info!(is_locked = command_is_lock, "wrote IsLocked");
        }
    }

    // Always report the result (02-REQ-5.4).
    let result_str = result.to_string();
    if let Err(e) = broker.set_lock_result(&result_str).await {
        // 02-REQ-5.E1: best-effort reporting.
        error!(error = %e, "failed to write LockResult");
    } else {
        info!(lock_result = %result_str, "wrote LockResult");
    }
}

/// Run the lock command handler loop.
///
/// Subscribes to `Vehicle.Command.Door.Lock` on the Kuksa Databroker and
/// processes each command through safety validation. This function runs
/// indefinitely until the subscription stream ends or an error occurs.
///
/// # Arguments
///
/// * `client` - A connected [`KuksaClient`] for subscribing to commands.
/// * `broker` - A [`DataBroker`] implementation for reading/writing signals.
/// * `config` - Service configuration (speed threshold, etc.).
pub async fn run_lock_handler<B: DataBroker>(
    client: &KuksaClient,
    broker: &B,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    use tokio_stream::StreamExt;

    info!("subscribing to {}", signals::COMMAND_DOOR_LOCK);
    let mut stream = client.subscribe_bool(signals::COMMAND_DOOR_LOCK).await?;
    info!("subscription established, waiting for lock commands");

    while let Some(command_result) = stream.next().await {
        match command_result {
            Ok(command_is_lock) => {
                process_lock_command(broker, command_is_lock, config.max_speed_kmh).await;
            }
            Err(e) => {
                error!(error = %e, "error receiving lock command from subscription");
                // Return error to trigger re-subscription in main loop.
                return Err(Box::new(e));
            }
        }
    }

    info!("lock command subscription stream ended");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Mock DataBroker for unit testing.
    ///
    /// Records all operations and returns configured values.
    #[derive(Debug, Clone)]
    struct MockBroker {
        speed: Option<f32>,
        door_is_open: Option<bool>,
        /// Recorded calls to `set_is_locked`.
        is_locked_writes: Arc<Mutex<Vec<bool>>>,
        /// Recorded calls to `set_lock_result`.
        lock_result_writes: Arc<Mutex<Vec<String>>>,
        /// If set, `set_is_locked` will return this error.
        set_is_locked_error: Option<String>,
        /// If set, `set_lock_result` will return this error.
        set_lock_result_error: Option<String>,
        /// If set, `get_speed` will return this error.
        get_speed_error: Option<String>,
        /// If set, `get_door_is_open` will return this error.
        get_door_error: Option<String>,
    }

    impl MockBroker {
        fn new() -> Self {
            Self {
                speed: Some(0.0),
                door_is_open: Some(false),
                is_locked_writes: Arc::new(Mutex::new(Vec::new())),
                lock_result_writes: Arc::new(Mutex::new(Vec::new())),
                set_is_locked_error: None,
                set_lock_result_error: None,
                get_speed_error: None,
                get_door_error: None,
            }
        }

        fn with_speed(mut self, speed: Option<f32>) -> Self {
            self.speed = speed;
            self
        }

        fn with_door_is_open(mut self, open: Option<bool>) -> Self {
            self.door_is_open = open;
            self
        }

        fn with_set_is_locked_error(mut self, err: &str) -> Self {
            self.set_is_locked_error = Some(err.to_string());
            self
        }

        fn with_set_lock_result_error(mut self, err: &str) -> Self {
            self.set_lock_result_error = Some(err.to_string());
            self
        }

        fn with_get_speed_error(mut self, err: &str) -> Self {
            self.get_speed_error = Some(err.to_string());
            self
        }

        fn with_get_door_error(mut self, err: &str) -> Self {
            self.get_door_error = Some(err.to_string());
            self
        }

        fn is_locked_writes(&self) -> Vec<bool> {
            self.is_locked_writes.lock().unwrap().clone()
        }

        fn lock_result_writes(&self) -> Vec<String> {
            self.lock_result_writes.lock().unwrap().clone()
        }
    }

    impl DataBroker for MockBroker {
        async fn get_speed(&self) -> Result<Option<f32>, BrokerError> {
            if let Some(ref err) = self.get_speed_error {
                return Err(BrokerError::Client(err.clone()));
            }
            Ok(self.speed)
        }

        async fn get_door_is_open(&self) -> Result<Option<bool>, BrokerError> {
            if let Some(ref err) = self.get_door_error {
                return Err(BrokerError::Client(err.clone()));
            }
            Ok(self.door_is_open)
        }

        async fn set_is_locked(&self, locked: bool) -> Result<(), BrokerError> {
            if let Some(ref err) = self.set_is_locked_error {
                return Err(BrokerError::Client(err.clone()));
            }
            self.is_locked_writes.lock().unwrap().push(locked);
            Ok(())
        }

        async fn set_lock_result(&self, result: &str) -> Result<(), BrokerError> {
            if let Some(ref err) = self.set_lock_result_error {
                return Err(BrokerError::Client(err.clone()));
            }
            self.lock_result_writes
                .lock()
                .unwrap()
                .push(result.to_string());
            Ok(())
        }
    }

    // ── Property 1: Command-Lock Invariant ──────────────────────────────
    // Safe conditions → IsLocked is written with commanded value.

    #[tokio::test]
    async fn lock_safe_conditions_writes_is_locked_true() {
        let broker = MockBroker::new().with_speed(Some(0.0)).with_door_is_open(Some(false));

        process_lock_command(&broker, true, 1.0).await;

        assert_eq!(broker.is_locked_writes(), vec![true]);
        assert_eq!(broker.lock_result_writes(), vec!["SUCCESS"]);
    }

    #[tokio::test]
    async fn unlock_safe_conditions_writes_is_locked_false() {
        let broker = MockBroker::new().with_speed(Some(0.0)).with_door_is_open(Some(false));

        process_lock_command(&broker, false, 1.0).await;

        assert_eq!(broker.is_locked_writes(), vec![false]);
        assert_eq!(broker.lock_result_writes(), vec!["SUCCESS"]);
    }

    // ── Property 2: Safety Rejection Guarantee ──────────────────────────
    // Rejected → IsLocked is NOT written.

    #[tokio::test]
    async fn lock_high_speed_does_not_write_is_locked() {
        let broker = MockBroker::new().with_speed(Some(50.0)).with_door_is_open(Some(false));

        process_lock_command(&broker, true, 1.0).await;

        assert!(broker.is_locked_writes().is_empty(), "IsLocked should NOT be written on speed rejection");
        assert_eq!(broker.lock_result_writes(), vec!["REJECTED_SPEED"]);
    }

    #[tokio::test]
    async fn unlock_high_speed_does_not_write_is_locked() {
        let broker = MockBroker::new().with_speed(Some(50.0)).with_door_is_open(Some(false));

        process_lock_command(&broker, false, 1.0).await;

        assert!(broker.is_locked_writes().is_empty());
        assert_eq!(broker.lock_result_writes(), vec!["REJECTED_SPEED"]);
    }

    #[tokio::test]
    async fn lock_door_open_does_not_write_is_locked() {
        let broker = MockBroker::new().with_speed(Some(0.0)).with_door_is_open(Some(true));

        process_lock_command(&broker, true, 1.0).await;

        assert!(broker.is_locked_writes().is_empty(), "IsLocked should NOT be written on door-open rejection");
        assert_eq!(broker.lock_result_writes(), vec!["REJECTED_DOOR_OPEN"]);
    }

    // ── 02-REQ-3.4: Unlock ignores door-ajar ───────────────────────────

    #[tokio::test]
    async fn unlock_door_open_succeeds() {
        let broker = MockBroker::new().with_speed(Some(0.0)).with_door_is_open(Some(true));

        process_lock_command(&broker, false, 1.0).await;

        assert_eq!(broker.is_locked_writes(), vec![false]);
        assert_eq!(broker.lock_result_writes(), vec!["SUCCESS"]);
    }

    // ── Property 3: Result Completeness ─────────────────────────────────
    // Exactly one LockResult for every command processed.

    #[tokio::test]
    async fn exactly_one_lock_result_per_command() {
        let broker = MockBroker::new();

        // Process three commands.
        process_lock_command(&broker, true, 1.0).await;
        process_lock_command(&broker, false, 1.0).await;
        process_lock_command(&broker, true, 1.0).await;

        assert_eq!(
            broker.lock_result_writes().len(),
            3,
            "expected exactly one LockResult per command"
        );
    }

    // ── Property 7: Default-Safe Behavior ───────────────────────────────
    // Missing signals use safe defaults.

    #[tokio::test]
    async fn missing_speed_treated_as_zero() {
        let broker = MockBroker::new().with_speed(None).with_door_is_open(Some(false));

        process_lock_command(&broker, true, 1.0).await;

        // Speed defaults to 0.0 → command should succeed.
        assert_eq!(broker.is_locked_writes(), vec![true]);
        assert_eq!(broker.lock_result_writes(), vec!["SUCCESS"]);
    }

    #[tokio::test]
    async fn missing_door_treated_as_closed() {
        let broker = MockBroker::new().with_speed(Some(0.0)).with_door_is_open(None);

        process_lock_command(&broker, true, 1.0).await;

        // Door defaults to closed → lock command should succeed.
        assert_eq!(broker.is_locked_writes(), vec![true]);
        assert_eq!(broker.lock_result_writes(), vec!["SUCCESS"]);
    }

    #[tokio::test]
    async fn both_signals_missing_treated_as_safe() {
        let broker = MockBroker::new().with_speed(None).with_door_is_open(None);

        process_lock_command(&broker, true, 1.0).await;

        assert_eq!(broker.is_locked_writes(), vec![true]);
        assert_eq!(broker.lock_result_writes(), vec!["SUCCESS"]);
    }

    // ── Error handling: get failures use safe defaults ───────────────────

    #[tokio::test]
    async fn speed_read_error_uses_safe_default() {
        let broker = MockBroker::new()
            .with_get_speed_error("connection lost")
            .with_door_is_open(Some(false));

        process_lock_command(&broker, true, 1.0).await;

        // Speed error → defaults to 0.0 → should succeed.
        assert_eq!(broker.is_locked_writes(), vec![true]);
        assert_eq!(broker.lock_result_writes(), vec!["SUCCESS"]);
    }

    #[tokio::test]
    async fn door_read_error_uses_safe_default() {
        let broker = MockBroker::new()
            .with_speed(Some(0.0))
            .with_get_door_error("connection lost");

        process_lock_command(&broker, true, 1.0).await;

        // Door error → defaults to closed → should succeed.
        assert_eq!(broker.is_locked_writes(), vec![true]);
        assert_eq!(broker.lock_result_writes(), vec!["SUCCESS"]);
    }

    // ── Error handling: write failures ───────────────────────────────────

    #[tokio::test]
    async fn is_locked_write_failure_still_reports_result() {
        // 02-REQ-4.E1: If writing IsLocked fails, still write LockResult.
        let broker = MockBroker::new()
            .with_speed(Some(0.0))
            .with_door_is_open(Some(false))
            .with_set_is_locked_error("write failed");

        process_lock_command(&broker, true, 1.0).await;

        // IsLocked write failed, so no successful writes recorded.
        assert!(broker.is_locked_writes().is_empty());
        // But LockResult should still be written.
        assert_eq!(broker.lock_result_writes(), vec!["SUCCESS"]);
    }

    #[tokio::test]
    async fn lock_result_write_failure_does_not_panic() {
        // 02-REQ-5.E1: best-effort reporting.
        let broker = MockBroker::new()
            .with_speed(Some(0.0))
            .with_door_is_open(Some(false))
            .with_set_lock_result_error("write failed");

        // Should not panic even if LockResult write fails.
        process_lock_command(&broker, true, 1.0).await;

        assert_eq!(broker.is_locked_writes(), vec![true]);
        assert!(broker.lock_result_writes().is_empty());
    }

    // ── Boundary: speed at threshold ────────────────────────────────────

    #[tokio::test]
    async fn speed_at_threshold_is_rejected() {
        let broker = MockBroker::new().with_speed(Some(1.0)).with_door_is_open(Some(false));

        process_lock_command(&broker, true, 1.0).await;

        assert!(broker.is_locked_writes().is_empty());
        assert_eq!(broker.lock_result_writes(), vec!["REJECTED_SPEED"]);
    }

    #[tokio::test]
    async fn speed_just_below_threshold_succeeds() {
        let broker = MockBroker::new().with_speed(Some(0.99)).with_door_is_open(Some(false));

        process_lock_command(&broker, true, 1.0).await;

        assert_eq!(broker.is_locked_writes(), vec![true]);
        assert_eq!(broker.lock_result_writes(), vec!["SUCCESS"]);
    }

    // ── BrokerError tests ───────────────────────────────────────────────

    #[test]
    fn broker_error_display() {
        let err = BrokerError::Client("test error".into());
        assert_eq!(err.to_string(), "databroker error: test error");
    }

    #[test]
    fn broker_error_debug() {
        let err = BrokerError::Client("test".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Client"));
    }
}
