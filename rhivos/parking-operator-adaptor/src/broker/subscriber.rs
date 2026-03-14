//! DATA_BROKER subscriber for lock/unlock events.
//!
//! [`BrokerSubscriber`] connects to the kuksa.val.v1 gRPC service and
//! subscribes to the `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` signal.
//!
//! Requirements: 08-REQ-6.1

use futures::stream::BoxStream;

/// VSS signal path for the driver-side door lock state.
pub const IS_LOCKED_SIGNAL: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";

/// Opaque lock event emitted by the subscriber stream.
pub struct LockEvent(pub bool);

/// Stub subscriber for the DATA_BROKER lock signal.
///
/// # Stub
/// Not yet connected to a real gRPC channel — task group 4 adds the kuksa
/// client implementation and retries per 08-REQ-6.E1.
pub struct BrokerSubscriber;

impl BrokerSubscriber {
    /// Connect to DATA_BROKER at `addr`.
    ///
    /// # Stub — always returns `Err` (task group 4)
    pub async fn connect(_addr: &str) -> Result<Self, String> {
        // STUB: task group 4 implements the real connection with retry.
        Err("BrokerSubscriber::connect not yet implemented".to_string())
    }

    /// Subscribe to `IS_LOCKED_SIGNAL` and return an event stream.
    ///
    /// # Stub — always returns `Err` (task group 4)
    pub async fn subscribe_lock_events(
        &mut self,
    ) -> Result<BoxStream<'static, Result<LockEvent, String>>, String> {
        // STUB: task group 4 implements the real gRPC subscribe call.
        Err("BrokerSubscriber::subscribe_lock_events not yet implemented".to_string())
    }

    /// Extract the `IsLocked` boolean from a lock event.
    pub fn extract_is_locked(event: &LockEvent) -> Option<bool> {
        Some(event.0)
    }
}
