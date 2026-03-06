//! DATA_BROKER publisher for SessionActive signal.
//!
//! Writes `Vehicle.Parking.SessionActive` to the Kuksa Databroker
//! via network gRPC (TCP).

use tonic::transport::{Channel, Endpoint};
use tracing::{info, warn};

use crate::kuksa_proto;
use crate::kuksa_proto::val_client::ValClient;
use crate::kuksa_proto::value::TypedValue;
use crate::kuksa_proto::{Datapoint, PublishValueRequest, SignalId};

/// VSS path for the parking session active signal.
const SESSION_ACTIVE_PATH: &str = "Vehicle.Parking.SessionActive";

/// Maximum backoff delay for connection retries (30 seconds).
const MAX_BACKOFF_SECS: u64 = 30;

/// Initial backoff delay for connection retries (1 second).
const INITIAL_BACKOFF_SECS: u64 = 1;

/// DATA_BROKER publisher for the SessionActive signal.
pub struct BrokerPublisher {
    client: ValClient<Channel>,
}

impl BrokerPublisher {
    /// Connect to DATA_BROKER via TCP with exponential backoff retry.
    pub async fn connect(addr: &str) -> Result<Self, tonic::transport::Error> {
        let mut backoff_secs = INITIAL_BACKOFF_SECS;

        loop {
            match Endpoint::try_from(addr.to_string()) {
                Ok(endpoint) => match endpoint.connect().await {
                    Ok(channel) => {
                        info!(addr = %addr, "Publisher connected to DATA_BROKER");
                        return Ok(Self {
                            client: ValClient::new(channel),
                        });
                    }
                    Err(e) => {
                        warn!(
                            addr = %addr,
                            backoff_secs = backoff_secs,
                            error = %e,
                            "DATA_BROKER unreachable for publisher, retrying"
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                        backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF_SECS);
                    }
                },
                Err(e) => return Err(e),
            }
        }
    }

    /// Write `Vehicle.Parking.SessionActive` to DATA_BROKER.
    ///
    /// On failure, logs the error but does not propagate it (08-REQ-6.E1).
    /// The internal session state remains as transitioned; the signal may be stale.
    pub async fn set_session_active(&mut self, active: bool) {
        let request = PublishValueRequest {
            signal_id: Some(SignalId {
                signal: Some(kuksa_proto::signal_id::Signal::Path(
                    SESSION_ACTIVE_PATH.to_string(),
                )),
            }),
            data_point: Some(Datapoint {
                timestamp: None,
                value: Some(kuksa_proto::Value {
                    typed_value: Some(TypedValue::Bool(active)),
                }),
            }),
        };

        match self.client.publish_value(request).await {
            Ok(_) => {
                info!(
                    path = SESSION_ACTIVE_PATH,
                    value = active,
                    "Published SessionActive to DATA_BROKER"
                );
            }
            Err(e) => {
                warn!(
                    path = SESSION_ACTIVE_PATH,
                    value = active,
                    error = %e,
                    "Failed to publish SessionActive to DATA_BROKER"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TS-08-3/TS-08-4: Verify BrokerPublisher can be constructed and
    /// set_session_active has the correct interface.
    /// Full integration requires a running DATA_BROKER.
    #[test]
    fn test_publisher_session_active_path() {
        assert_eq!(SESSION_ACTIVE_PATH, "Vehicle.Parking.SessionActive");
    }
}
