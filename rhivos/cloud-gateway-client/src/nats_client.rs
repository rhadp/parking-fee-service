use std::time::Duration;

use tracing::warn;

/// Trait abstracting NATS publish operations for testability.
#[allow(async_fn_in_trait)]
pub trait NatsPublisher {
    /// Publish a payload to a NATS subject.
    async fn publish(&self, subject: &str, payload: &[u8]) -> Result<(), NatsError>;
}

/// Error type for NATS operations.
#[derive(Debug, Clone)]
pub struct NatsError(pub String);

impl std::fmt::Display for NatsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NatsError: {}", self.0)
    }
}

impl std::error::Error for NatsError {}

/// A NATS client wrapping `async_nats::Client`.
pub struct NatsClient {
    inner: async_nats::Client,
}

/// A NATS subscription wrapping `async_nats::Subscriber`.
pub struct NatsSubscription {
    inner: async_nats::Subscriber,
}

impl NatsClient {
    /// Connect to NATS with exponential backoff retry.
    ///
    /// Makes up to 5 attempts with delays of 1s, 2s, 4s, 8s between them.
    /// Returns `Err` if all 5 initial attempts fail (04-REQ-1.E1).
    ///
    /// Post-connection reconnect behaviour (04-REQ-1.E2) is delegated to the
    /// async-nats client, which retries reconnections automatically by default.
    pub async fn connect(url: &str) -> Result<Self, NatsError> {
        let mut delay = Duration::from_secs(1);
        for attempt in 1..=5u32 {
            match async_nats::connect(url).await {
                Ok(client) => return Ok(NatsClient { inner: client }),
                Err(e) => {
                    if attempt == 5 {
                        return Err(NatsError(format!(
                            "Failed to connect to NATS after 5 attempts: {}",
                            e
                        )));
                    }
                    warn!(
                        attempt,
                        retry_in_secs = delay.as_secs(),
                        error = %e,
                        "NATS connection attempt failed, retrying"
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
            }
        }
        unreachable!()
    }

    /// Subscribe to a NATS subject.
    pub async fn subscribe(&self, subject: &str) -> Result<NatsSubscription, NatsError> {
        self.inner
            .subscribe(subject.to_string())
            .await
            .map(|sub| NatsSubscription { inner: sub })
            .map_err(|e| NatsError(format!("Failed to subscribe to '{}': {}", subject, e)))
    }

    /// Flush any pending outbound messages (used during graceful shutdown).
    pub async fn flush(&self) -> Result<(), NatsError> {
        self.inner
            .flush()
            .await
            .map_err(|e| NatsError(format!("NATS flush failed: {}", e)))
    }
}

impl NatsPublisher for NatsClient {
    async fn publish(&self, subject: &str, payload: &[u8]) -> Result<(), NatsError> {
        self.inner
            .publish(subject.to_string(), payload.to_vec().into())
            .await
            .map_err(|e| NatsError(format!("NATS publish to '{}' failed: {}", subject, e)))
    }
}

impl NatsSubscription {
    /// Receive the next message from this subscription.
    /// Returns `None` when the subscription is closed.
    pub async fn next(&mut self) -> Option<async_nats::Message> {
        // `Subscriber` implements `futures::Stream`; pull in `StreamExt` locally.
        use futures::StreamExt;
        self.inner.next().await
    }
}
