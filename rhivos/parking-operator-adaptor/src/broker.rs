//! DATA_BROKER client abstraction.
//!
//! Wraps a Kuksa Databroker gRPC client (kuksa.val.v1) and provides typed
//! subscribe/set operations on VSS signals.

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

// ── VSS signal path constants ────────────────────────────────────────────────

/// IsLocked signal subscribed by the adaptor for autonomous session management.
pub const SIGNAL_IS_LOCKED: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";
/// SessionActive signal published after each session start or stop.
pub const SIGNAL_SESSION_ACTIVE: &str = "Vehicle.Parking.SessionActive";

// ── Errors ────────────────────────────────────────────────────────────────────

/// Errors produced by DATA_BROKER operations.
#[derive(Debug, Clone)]
pub enum BrokerError {
    /// DATA_BROKER is unreachable or returned a gRPC error.
    Unavailable(String),
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerError::Unavailable(msg) => write!(f, "broker unavailable: {msg}"),
        }
    }
}

// ── SessionPublisher trait ────────────────────────────────────────────────────

/// Trait for publishing `Vehicle.Parking.SessionActive` to DATA_BROKER.
///
/// Implementations must be `Send + Sync` to support trait-object usage across
/// async task boundaries.
#[async_trait]
pub trait SessionPublisher: Send + Sync {
    /// Set `Vehicle.Parking.SessionActive` to `active` in DATA_BROKER.
    ///
    /// Failure must be tolerated: log the error and continue (08-REQ-4.E1).
    async fn set_session_active(&self, active: bool) -> Result<(), BrokerError>;
}

// ── Proto ─────────────────────────────────────────────────────────────────────

mod proto {
    pub mod kuksa {
        pub mod val {
            pub mod v1 {
                tonic::include_proto!("kuksa.val.v1");
            }
        }
    }
}

use proto::kuksa::val::v1::{
    datapoint, val_service_client::ValServiceClient, DataEntry, Datapoint, EntryUpdate, Field,
    SetRequest, SubscribeEntry, SubscribeRequest,
};

// ── BrokerClient ─────────────────────────────────────────────────────────────

/// Concrete DATA_BROKER client backed by the Kuksa gRPC (kuksa.val.v1) service.
pub struct BrokerClient {
    client: ValServiceClient<tonic::transport::Channel>,
}

impl BrokerClient {
    /// Connect to DATA_BROKER at `addr`, retrying with exponential backoff.
    ///
    /// Attempts connection up to 5 times with delays of 1 s, 2 s, 4 s between
    /// attempts. The last two retries repeat the 4 s delay. Returns
    /// `Err(BrokerError::Unavailable)` after all attempts fail; the caller
    /// (main) is responsible for exiting with a non-zero code (08-REQ-3.E3).
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        // 5 total attempts, delays before attempts 2-5: 1s, 2s, 4s, 4s
        let retry_delays_secs: &[u64] = &[1, 2, 4, 4];
        let mut last_err = BrokerError::Unavailable("no attempts made".to_string());

        for attempt in 0..5usize {
            if attempt > 0 {
                let delay = retry_delays_secs[attempt - 1];
                warn!(attempt, delay, "DATA_BROKER connection failed, retrying");
                tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
            }

            match ValServiceClient::connect(addr.to_string()).await {
                Ok(client) => {
                    info!(addr, "Connected to DATA_BROKER");
                    return Ok(BrokerClient { client });
                }
                Err(e) => {
                    let msg = e.to_string();
                    warn!(attempt = attempt + 1, error = %msg, "DATA_BROKER connection attempt failed");
                    last_err = BrokerError::Unavailable(msg);
                }
            }
        }

        Err(last_err)
    }

    /// Subscribe to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` and return a
    /// channel receiver that yields bool values as they arrive.
    ///
    /// A background task reads the gRPC stream and forwards `bool_value` updates.
    ///
    /// Validates: 08-REQ-3.2, 08-REQ-3.3, 08-REQ-3.4
    pub async fn subscribe_is_locked(&self) -> Result<mpsc::Receiver<bool>, BrokerError> {
        let request = SubscribeRequest {
            entries: vec![SubscribeEntry {
                path: SIGNAL_IS_LOCKED.to_string(),
                fields: vec![Field::Value as i32],
            }],
        };

        let mut client = self.client.clone();
        let mut stream = client
            .subscribe(request)
            .await
            .map_err(|e| {
                error!(signal = SIGNAL_IS_LOCKED, error = %e, "Failed to subscribe to IsLocked");
                BrokerError::Unavailable(e.to_string())
            })?
            .into_inner();

        let (tx, rx) = mpsc::channel::<bool>(16);

        tokio::spawn(async move {
            loop {
                match stream.message().await {
                    Ok(Some(response)) => {
                        for update in response.updates {
                            if let Some(DataEntry {
                                value:
                                    Some(Datapoint {
                                        value: Some(datapoint::Value::Bool(b)),
                                    }),
                                ..
                            }) = update.entry
                            {
                                if tx.send(b).await.is_err() {
                                    // Receiver dropped — exit background task.
                                    return;
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        error!(signal = SIGNAL_IS_LOCKED, "IsLocked subscribe stream ended unexpectedly");
                        return;
                    }
                    Err(e) => {
                        error!(signal = SIGNAL_IS_LOCKED, error = %e, "IsLocked subscribe stream error");
                        return;
                    }
                }
            }
        });

        info!(signal = SIGNAL_IS_LOCKED, "Subscribed to IsLocked signal");
        Ok(rx)
    }
}

// ── SessionPublisher impl ─────────────────────────────────────────────────────

#[async_trait]
impl SessionPublisher for BrokerClient {
    /// Set `Vehicle.Parking.SessionActive` in DATA_BROKER.
    ///
    /// Errors are returned so the caller can log and continue (08-REQ-4.E1).
    async fn set_session_active(&self, active: bool) -> Result<(), BrokerError> {
        let request = SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: SIGNAL_SESSION_ACTIVE.to_string(),
                    value: Some(Datapoint {
                        value: Some(datapoint::Value::Bool(active)),
                    }),
                }),
                fields: vec![Field::Value as i32],
            }],
        };

        let mut client = self.client.clone();
        client
            .set(request)
            .await
            .map_err(|e| {
                error!(
                    signal = SIGNAL_SESSION_ACTIVE,
                    active,
                    error = %e,
                    "Failed to publish SessionActive to DATA_BROKER"
                );
                BrokerError::Unavailable(e.to_string())
            })?;

        info!(signal = SIGNAL_SESSION_ACTIVE, active, "Published SessionActive to DATA_BROKER");
        Ok(())
    }
}
