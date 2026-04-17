//! DATA_BROKER gRPC client (Kuksa Databroker VAL service).
//!
//! Connects to DATA_BROKER at startup with exponential-backoff retry (08-REQ-3.E3),
//! subscribes to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` (08-REQ-3.2),
//! and publishes `Vehicle.Parking.SessionActive` (08-REQ-4.1, 08-REQ-4.2, 08-REQ-4.3).

use crate::event_loop::{BrokerError, BrokerTrait};
use crate::proto::kuksa::{
    self as kuksa_pb, val_client::ValClient, DataEntry, Datapoint, EntryUpdate, Field, SetRequest,
    SubscribeEntry, SubscribeRequest, View,
};
use tonic::transport::Channel;

/// Stream of raw subscribe responses from DATA_BROKER.
pub type SubscribeStream = tonic::Streaming<kuksa_pb::SubscribeResponse>;

/// DATA_BROKER gRPC client wrapping the kuksa.val.v1 VAL service.
///
/// The inner `ValClient<Channel>` is cheap to clone — all clones share the
/// same underlying HTTP/2 connection.
pub struct BrokerClient {
    inner: ValClient<Channel>,
}

impl BrokerClient {
    /// Connect to DATA_BROKER at `addr` with exponential-backoff retry.
    ///
    /// Attempts: up to 5, inter-attempt delays: 1 s, 2 s, 4 s, 8 s.
    /// Returns `BrokerError` after all attempts fail (08-REQ-3.E3).
    pub async fn connect(addr: &str) -> Result<Self, BrokerError> {
        const MAX_ATTEMPTS: usize = 5;
        const DELAYS_SECS: [u64; 4] = [1, 2, 4, 8];

        let mut last_err = String::from("no attempts made");

        for attempt in 0..MAX_ATTEMPTS {
            if attempt > 0 {
                let delay = DELAYS_SECS[attempt - 1];
                tracing::warn!(
                    "DATA_BROKER connection retry {}/{MAX_ATTEMPTS} in {delay}s",
                    attempt + 1
                );
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            }

            match ValClient::connect(addr.to_string()).await {
                Ok(client) => {
                    tracing::info!("Connected to DATA_BROKER at {addr}");
                    return Ok(BrokerClient { inner: client });
                }
                Err(e) => {
                    tracing::warn!(
                        "DATA_BROKER connection attempt {}/{MAX_ATTEMPTS} failed: {e}",
                        attempt + 1
                    );
                    last_err = e.to_string();
                }
            }
        }

        Err(BrokerError(format!(
            "DATA_BROKER unreachable after {MAX_ATTEMPTS} attempts: {last_err}"
        )))
    }

    /// Subscribe to a bool VSS signal (08-REQ-3.2).
    ///
    /// Returns a raw subscribe response stream; callers extract the bool from
    /// `SubscribeResponse::updates[].entry.value.bool_value`.
    pub async fn subscribe_bool(
        &mut self,
        signal: &str,
    ) -> Result<SubscribeStream, BrokerError> {
        let req = SubscribeRequest {
            entries: vec![SubscribeEntry {
                path: signal.to_string(),
                view: View::CurrentValue as i32,
                fields: vec![Field::Value as i32],
            }],
        };

        self.inner
            .subscribe(req)
            .await
            .map_err(|e| BrokerError(format!("DATA_BROKER subscribe failed: {e}")))
            .map(|r| r.into_inner())
    }
}

impl BrokerTrait for BrokerClient {
    /// Set a bool VSS signal in DATA_BROKER via `Set` RPC (08-REQ-4.1, 08-REQ-4.2, 08-REQ-4.3).
    async fn set_bool(&self, signal: &str, value: bool) -> Result<(), BrokerError> {
        // Clone is cheap — shares the underlying HTTP/2 connection.
        let mut client = self.inner.clone();

        let req = SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: signal.to_string(),
                    value: Some(Datapoint {
                        timestamp: 0,
                        value: Some(kuksa_pb::datapoint::Value::BoolValue(value)),
                    }),
                    actuator_target: None,
                    metadata: None,
                }),
                fields: vec![Field::Value as i32],
            }],
        };

        let resp = client
            .set(req)
            .await
            .map_err(|e| BrokerError(format!("DATA_BROKER Set RPC failed: {e}")))?
            .into_inner();

        if !resp.errors.is_empty() {
            tracing::warn!(
                "DATA_BROKER Set({signal}={value}) returned {} error(s)",
                resp.errors.len()
            );
        }

        Ok(())
    }
}
