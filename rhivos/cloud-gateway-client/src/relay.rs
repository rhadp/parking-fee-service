use tracing::error;

use crate::broker::{BrokerClient, BrokerError};
use crate::nats_client::NatsPublisher;
use crate::telemetry::{build_telemetry, TelemetryState};

/// VSS signal path for the lock command.
pub const SIGNAL_LOCK_COMMAND: &str = "Vehicle.Command.Door.Lock";

/// Relay a command response JSON string verbatim to NATS.
///
/// Publishes to `vehicles.{vin}.command_responses`. If the publish fails,
/// logs the error and returns without propagating (04-REQ-3.E1).
pub async fn relay_response<N: NatsPublisher>(nats: &N, vin: &str, response_json: &str) {
    let subject = format!("vehicles.{vin}.command_responses");
    if let Err(e) = nats.publish(&subject, response_json.as_bytes()).await {
        error!("Failed to publish command response to NATS: {}", e);
    }
}

/// Forward a validated command JSON string to DATA_BROKER.
///
/// Sets the `Vehicle.Command.Door.Lock` signal to the given JSON value.
/// Returns an error if the broker set operation fails.
pub async fn forward_command<B: BrokerClient>(
    broker: &B,
    command_json: &str,
) -> Result<(), BrokerError> {
    broker.set_string(SIGNAL_LOCK_COMMAND, command_json).await
}

/// Publish an aggregated telemetry message to NATS.
///
/// Builds and publishes a telemetry JSON message to `vehicles.{vin}.telemetry`.
/// If the publish fails, logs the error and returns without propagating (04-REQ-4.E2).
pub async fn publish_telemetry<N: NatsPublisher>(nats: &N, vin: &str, state: &TelemetryState) {
    let msg = build_telemetry(vin, state);
    let payload = match serde_json::to_vec(&msg) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to serialize telemetry message: {}", e);
            return;
        }
    };
    let subject = format!("vehicles.{vin}.telemetry");
    if let Err(e) = nats.publish(&subject, &payload).await {
        error!("Failed to publish telemetry to NATS: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{MockBrokerClient, MockNatsPublisher};

    // TS-04-8: Response relay is verbatim
    #[tokio::test]
    async fn test_response_relay_verbatim() {
        let mock_nats = MockNatsPublisher::new();
        let response_json = r#"{"command_id":"abc-123","status":"success","timestamp":1700000001}"#;

        relay_response(&mock_nats, "WDB123", response_json).await;

        let (subject, payload) = mock_nats
            .last_publish()
            .expect("should have published one message");
        assert_eq!(subject, "vehicles.WDB123.command_responses");
        assert_eq!(
            std::str::from_utf8(&payload).unwrap(),
            response_json,
            "published payload should match input verbatim"
        );
    }

    // TS-04-6: Command forwarding to DATA_BROKER
    #[tokio::test]
    async fn test_command_forwarding() {
        let mock_broker = MockBrokerClient::new();
        let command_json =
            r#"{"command_id":"abc-123","action":"lock","doors":["driver"]}"#;

        let result = forward_command(&mock_broker, command_json).await;
        assert!(result.is_ok(), "forward_command should succeed");

        let calls = mock_broker.set_string_calls();
        assert_eq!(calls.len(), 1, "should have called set_string once");
        let (signal, value) = &calls[0];
        assert_eq!(signal, SIGNAL_LOCK_COMMAND);
        assert!(
            value.contains("abc-123"),
            "published value should contain the command_id"
        );
    }

    // TS-04-E6: Response NATS publish failure — service continues
    #[tokio::test]
    async fn test_response_publish_failure() {
        let mock_nats = MockNatsPublisher::new();
        mock_nats.fail_next_publish();

        let response_json = r#"{"command_id":"abc-123","status":"success","timestamp":1700000001}"#;
        // Should not panic — failure is logged and swallowed (04-REQ-3.E1)
        relay_response(&mock_nats, "VIN", response_json).await;

        // Confirm that the failure was encountered (not silently skipped)
        assert_eq!(
            mock_nats.failure_count(),
            1,
            "one publish failure should have been handled"
        );

        // Confirm service can still publish after a failure
        relay_response(&mock_nats, "VIN", response_json).await;
        assert_eq!(
            mock_nats.publishes().len(),
            1,
            "second relay should succeed"
        );
    }

    // TS-04-E8: Telemetry NATS publish failure — service continues
    #[tokio::test]
    async fn test_telemetry_publish_failure() {
        let mock_nats = MockNatsPublisher::new();
        mock_nats.fail_next_publish();

        let state = TelemetryState {
            is_locked: Some(true),
            latitude: Some(48.85),
            longitude: Some(2.35),
            parking_active: None,
        };
        // Should not panic — failure is logged and swallowed (04-REQ-4.E2)
        publish_telemetry(&mock_nats, "VIN", &state).await;

        assert_eq!(
            mock_nats.failure_count(),
            1,
            "one publish failure should have been handled"
        );

        // Confirm service can still publish telemetry after a failure
        publish_telemetry(&mock_nats, "VIN", &state).await;
        assert_eq!(
            mock_nats.publishes().len(),
            1,
            "second telemetry publish should succeed"
        );
    }
}
