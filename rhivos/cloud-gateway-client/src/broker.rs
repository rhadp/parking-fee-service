use std::time::Duration;

use tokio::sync::mpsc;
use tracing::warn;

/// Trait abstracting the DATA_BROKER gRPC client for testability.
#[allow(async_fn_in_trait)]
pub trait BrokerClient {
    /// Set a string-valued signal in DATA_BROKER.
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError>;
}

/// Error type for broker operations.
#[derive(Debug, Clone)]
pub struct BrokerError(pub String);

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BrokerError: {}", self.0)
    }
}

impl std::error::Error for BrokerError {}

/// An update notification received from a DATA_BROKER signal subscription.
#[derive(Debug, Clone)]
pub struct BrokerUpdate {
    /// The VSS signal path that changed.
    pub path: String,
    /// The new value of the signal.
    pub value: BrokerValue,
}

/// The value of a DATA_BROKER signal.
///
/// Variants are matched in the main task loop; they will be *constructed* by the
/// real gRPC `Subscribe` streaming handler added in task group 6.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum BrokerValue {
    /// A string-valued signal (used for command/response signals).
    String(String),
    /// A boolean-valued signal (used for lock status, parking state).
    Bool(bool),
    /// A floating-point signal (used for latitude, longitude).
    Float(f64),
}

/// Concrete gRPC DATA_BROKER client.
///
/// Wraps a tonic transport channel to the Kuksa Databroker.  Signal writes use
/// the `kuksa.val.v1.VAL/Set` RPC; subscriptions use `VAL/Subscribe`.
///
/// Note: The full gRPC implementation of `set_string` and `subscribe_signals`
/// requires generated proto types from `kuksa.val.v1`.  That code generation is
/// set up in task group 6 (integration tests).  For now `set_string` succeeds
/// immediately (logged at debug level) and `subscribe_signals` returns an empty
/// receiver that will close once the returned sender is dropped.
pub struct GrpcBrokerClient {
    addr: String,
    // The channel is kept alive so that the endpoint remains usable.
    #[allow(dead_code)]
    channel: tonic::transport::Channel,
}

impl GrpcBrokerClient {
    /// Connect to DATA_BROKER with exponential backoff retry.
    ///
    /// Makes up to 5 attempts with delays of 1s, 2s, 4s, 8s between them.
    /// Returns `Err` if all 5 attempts fail (04-REQ-5.E1).
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        let mut delay = Duration::from_secs(1);
        for attempt in 1..=5u32 {
            let endpoint =
                tonic::transport::Endpoint::from_shared(addr.to_string()).map_err(|e| {
                    BrokerError(format!("Invalid DATA_BROKER address '{}': {}", addr, e))
                })?;

            match endpoint.connect().await {
                Ok(channel) => {
                    return Ok(GrpcBrokerClient {
                        addr: addr.to_string(),
                        channel,
                    })
                }
                Err(e) => {
                    if attempt == 5 {
                        return Err(BrokerError(format!(
                            "Failed to connect to DATA_BROKER at '{}' after {} attempts: {}",
                            addr, attempt, e
                        )));
                    }
                    warn!(
                        attempt,
                        retry_in_secs = delay.as_secs(),
                        error = %e,
                        "DATA_BROKER connection attempt failed, retrying"
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
            }
        }
        unreachable!()
    }

    /// Subscribe to a list of DATA_BROKER signals.
    ///
    /// Returns a `Receiver` that yields `BrokerUpdate` whenever a subscribed
    /// signal changes.
    ///
    /// # Current implementation
    ///
    /// This is a structural placeholder: the returned `Receiver` will yield
    /// `None` immediately (the channel sender is dropped on return).  The real
    /// `kuksa.val.v1.VAL/Subscribe` gRPC streaming call is wired up in task
    /// group 6, once proto code generation is available.
    pub async fn subscribe_signals(&self, signals: &[&str]) -> mpsc::Receiver<BrokerUpdate> {
        let (_tx, rx) = mpsc::channel(100);
        tracing::info!(
            addr = %self.addr,
            signals = ?signals,
            "DATA_BROKER signal subscription registered (gRPC proto pending, task group 6)"
        );
        // _tx is intentionally dropped here.  When task group 6 adds the real
        // gRPC streaming call, _tx will be moved into a spawned task that drives
        // the server-streaming response.
        rx
    }
}

impl BrokerClient for GrpcBrokerClient {
    /// Set a string-valued VSS signal in DATA_BROKER.
    ///
    /// # Current implementation
    ///
    /// Logs the call and returns `Ok(())`.  The real `kuksa.val.v1.VAL/Set`
    /// gRPC call is wired up in task group 6, once proto code generation is
    /// available.
    async fn set_string(&self, signal: &str, value: &str) -> Result<(), BrokerError> {
        tracing::debug!(
            addr = %self.addr,
            signal,
            value,
            "DATA_BROKER set_string (gRPC proto pending, task group 6)"
        );
        Ok(())
    }
}
