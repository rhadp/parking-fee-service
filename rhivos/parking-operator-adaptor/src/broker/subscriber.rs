//! DATA_BROKER subscriber for lock/unlock events.
//!
//! Subscribes to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` on the
//! Kuksa Databroker via network gRPC (TCP) and exposes a stream of bool values.

use tokio_stream::Stream;
use tonic::transport::{Channel, Endpoint};
use tracing::{info, warn};

use crate::kuksa_proto::val_client::ValClient;
use crate::kuksa_proto::value::TypedValue;
use crate::kuksa_proto::SubscribeRequest;

/// VSS path for the driver-side door lock state.
const IS_LOCKED_PATH: &str = "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked";

/// Maximum backoff delay for connection retries (30 seconds).
const MAX_BACKOFF_SECS: u64 = 30;

/// Initial backoff delay for connection retries (1 second).
const INITIAL_BACKOFF_SECS: u64 = 1;

/// DATA_BROKER subscriber for lock/unlock events.
pub struct BrokerSubscriber {
    addr: String,
}

impl BrokerSubscriber {
    /// Creates a new BrokerSubscriber targeting the given DATA_BROKER address.
    pub fn new(addr: &str) -> Self {
        Self {
            addr: addr.to_string(),
        }
    }

    /// Connect to DATA_BROKER with exponential backoff retry.
    async fn connect(&self) -> Result<ValClient<Channel>, tonic::transport::Error> {
        let mut backoff_secs = INITIAL_BACKOFF_SECS;

        loop {
            match Endpoint::try_from(self.addr.clone()) {
                Ok(endpoint) => match endpoint.connect().await {
                    Ok(channel) => {
                        info!(addr = %self.addr, "Connected to DATA_BROKER");
                        return Ok(ValClient::new(channel));
                    }
                    Err(e) => {
                        warn!(
                            addr = %self.addr,
                            backoff_secs = backoff_secs,
                            error = %e,
                            "DATA_BROKER unreachable, retrying"
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                        backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF_SECS);
                    }
                },
                Err(e) => return Err(e),
            }
        }
    }

    /// Subscribe to lock/unlock events from DATA_BROKER.
    ///
    /// Returns a stream of boolean values where `true` means locked
    /// and `false` means unlocked. The stream yields values as they
    /// are published to the `IsLocked` signal on DATA_BROKER.
    ///
    /// This method retries the connection with exponential backoff if
    /// the DATA_BROKER is unreachable at startup (08-REQ-2.E1).
    pub async fn subscribe_lock_events(
        &self,
    ) -> Result<impl Stream<Item = bool>, Box<dyn std::error::Error + Send>> {
        let mut client = self
            .connect()
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;

        let request = SubscribeRequest {
            signal_paths: vec![IS_LOCKED_PATH.to_string()],
            buffer_size: 0,
            filter: None,
        };

        let response = client
            .subscribe(request)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send> { Box::new(e) })?;

        let stream = response.into_inner();

        let mapped = async_stream::stream! {
            use tokio_stream::StreamExt;
            let mut stream = stream;
            while let Some(result) = stream.next().await {
                match result {
                    Ok(subscribe_response) => {
                        for (path, datapoint) in &subscribe_response.entries {
                            if path == IS_LOCKED_PATH {
                                if let Some(value) = &datapoint.value {
                                    if let Some(TypedValue::Bool(locked)) = &value.typed_value {
                                        yield *locked;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "DATA_BROKER subscription error");
                        break;
                    }
                }
            }
        };

        Ok(mapped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify the IsLocked VSS path constant is correct per design.
    #[test]
    fn test_subscriber_is_locked_path() {
        assert_eq!(
            IS_LOCKED_PATH,
            "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
        );
    }

    /// Verify BrokerSubscriber can be constructed with an address.
    #[test]
    fn test_subscriber_new() {
        let sub = BrokerSubscriber::new("http://localhost:55556");
        assert_eq!(sub.addr, "http://localhost:55556");
    }
}
