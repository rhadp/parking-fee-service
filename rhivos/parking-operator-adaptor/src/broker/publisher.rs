//! DATA_BROKER publisher for `Vehicle.Parking.SessionActive`.
//!
//! [`BrokerSessionPublisher`] implements [`super::SessionPublisher`] by
//! writing the signal over a kuksa.val.v1 gRPC channel.
//!
//! Requirements: 08-REQ-6.2, 08-REQ-6.E2

use super::SessionPublisher;
use crate::kuksav1;

/// VSS signal path for parking session active state.
const SESSION_ACTIVE_SIGNAL: &str = "Vehicle.Parking.SessionActive";

/// Concrete publisher that writes `Vehicle.Parking.SessionActive` to DATA_BROKER.
///
/// Holds a shared `tonic::transport::Channel` (internally Arc-based) so the
/// struct can be cloned cheaply.  Each `set_session_active` call creates a
/// fresh `ValClient` on the shared channel.
pub struct BrokerSessionPublisher {
    channel: tonic::transport::Channel,
}

impl BrokerSessionPublisher {
    /// Create a new `BrokerSessionPublisher` connected to DATA_BROKER at `addr`.
    pub async fn connect(addr: &str) -> Result<Self, String> {
        let endpoint = tonic::transport::Endpoint::from_shared(addr.to_string())
            .map_err(|e| format!("invalid DATA_BROKER address '{}': {}", addr, e))?;
        let channel = endpoint
            .connect()
            .await
            .map_err(|e| format!("failed to connect to DATA_BROKER at '{}': {}", addr, e))?;
        Ok(Self { channel })
    }

    /// Create a `BrokerSessionPublisher` from an existing channel.
    pub fn from_channel(channel: tonic::transport::Channel) -> Self {
        Self { channel }
    }
}

#[tonic::async_trait]
impl SessionPublisher for BrokerSessionPublisher {
    /// Write `Vehicle.Parking.SessionActive` to DATA_BROKER (08-REQ-6.2).
    ///
    /// On failure, returns an error string.  Callers should log the error and
    /// continue operating — session state is authoritative (08-REQ-6.E2).
    async fn set_session_active(&self, active: bool) -> Result<(), String> {
        use kuksav1::{datapoint, val_client::ValClient, DataEntry, Datapoint, EntryUpdate, Field, SetRequest};

        let mut client = ValClient::new(self.channel.clone());

        let req = tonic::Request::new(SetRequest {
            updates: vec![EntryUpdate {
                entry: Some(DataEntry {
                    path: SESSION_ACTIVE_SIGNAL.to_string(),
                    value: Some(Datapoint {
                        value: Some(datapoint::Value::Bool(active)),
                    }),
                    actuator_target: None,
                }),
                fields: vec![Field::Value as i32],
            }],
        });

        client
            .set(req)
            .await
            .map_err(|e| format!("set_session_active({}) RPC failed: {}", active, e))?;

        Ok(())
    }
}
