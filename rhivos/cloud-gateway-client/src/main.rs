use std::collections::HashMap;
use std::process::ExitCode;

use cloud_gateway_client::broker_client::BrokerClient;
use cloud_gateway_client::command_validator::{validate_bearer_token, validate_command_payload};
use cloud_gateway_client::config::Config;
use cloud_gateway_client::nats_client::NatsClient;
use cloud_gateway_client::telemetry::TelemetryState;

#[tokio::main]
async fn main() -> ExitCode {
    // 04-REQ-10.1: Initialize tracing subscriber for structured logging.
    tracing_subscriber::fmt::init();

    // 04-REQ-9.1: Startup sequence step 1 — read and validate env vars.
    let config = match Config::from_env() {
        Ok(cfg) => {
            tracing::info!(vin = %cfg.vin, "Configuration loaded");
            cfg
        }
        Err(e) => {
            tracing::error!(error = %e, "Configuration error");
            return ExitCode::from(1);
        }
    };

    // 04-REQ-9.1: Startup sequence step 2 — connect to NATS.
    let nats = match NatsClient::connect(&config).await {
        Ok(client) => client,
        Err(e) => {
            tracing::error!(error = %e, "Failed to connect to NATS");
            return ExitCode::from(1);
        }
    };

    // 04-REQ-9.1: Startup sequence step 3 — connect to DATA_BROKER.
    let broker = match BrokerClient::connect(&config).await {
        Ok(client) => client,
        Err(e) => {
            tracing::error!(error = %e, "Failed to connect to DATA_BROKER");
            return ExitCode::from(1);
        }
    };

    // 04-REQ-9.1: Startup sequence step 4 — publish self-registration.
    if let Err(e) = nats.publish_registration().await {
        tracing::error!(error = %e, "Failed to publish self-registration");
        return ExitCode::from(1);
    }

    // 04-REQ-9.1: Startup sequence step 5 — begin processing.
    // Subscribe to DATA_BROKER signals before spawning loops.
    let mut response_rx = match broker.subscribe_responses().await {
        Ok(rx) => rx,
        Err(e) => {
            tracing::error!(error = %e, "Failed to subscribe to command responses");
            return ExitCode::from(1);
        }
    };

    let mut telemetry_rx = match broker.subscribe_telemetry().await {
        Ok(rx) => rx,
        Err(e) => {
            tracing::error!(error = %e, "Failed to subscribe to telemetry signals");
            return ExitCode::from(1);
        }
    };

    let mut command_sub = match nats.subscribe_commands().await {
        Ok(sub) => sub,
        Err(e) => {
            tracing::error!(error = %e, "Failed to subscribe to NATS commands");
            return ExitCode::from(1);
        }
    };

    tracing::info!("Service started, processing commands and telemetry");

    // Use Arc to share broker and nats across tasks.
    let broker = std::sync::Arc::new(broker);
    let nats = std::sync::Arc::new(nats);
    let bearer_token = config.bearer_token.clone();

    // Spawn command processing loop.
    let broker_cmd = broker.clone();
    let command_handle = tokio::spawn(async move {
        use futures::StreamExt;
        while let Some(msg) = command_sub.next().await {
            // 04-REQ-5.1: Extract Authorization header from NATS message.
            let headers = extract_headers(&msg);

            // 04-REQ-5.2: Validate bearer token.
            if let Err(e) = validate_bearer_token(&headers, &bearer_token) {
                tracing::warn!(error = %e, "Command authentication failed");
                continue;
            }

            // 04-REQ-6.1: Validate command payload.
            let payload = &msg.payload;
            if let Err(e) = validate_command_payload(payload) {
                tracing::warn!(error = %e, "Command validation failed");
                continue;
            }

            // 04-REQ-6.3: Write command payload as-is to DATA_BROKER.
            let payload_str = match std::str::from_utf8(payload) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(error = %e, "Command payload is not valid UTF-8");
                    continue;
                }
            };

            if let Err(e) = broker_cmd.write_command(payload_str).await {
                tracing::error!(error = %e, "Failed to write command to DATA_BROKER");
            } else {
                tracing::info!("Validated command forwarded to DATA_BROKER");
            }
        }
        tracing::info!("Command processing loop ended");
    });

    // Spawn response relay loop.
    let nats_rsp = nats.clone();
    let response_handle = tokio::spawn(async move {
        while let Some(response_json) = response_rx.recv().await {
            // 04-REQ-7.E1: Validate that response is valid JSON before relaying.
            if serde_json::from_str::<serde_json::Value>(&response_json).is_err() {
                tracing::error!("Invalid JSON in command response from DATA_BROKER, skipping");
                continue;
            }

            // 04-REQ-7.1: Publish response verbatim to NATS.
            if let Err(e) = nats_rsp.publish_response(&response_json).await {
                tracing::error!(error = %e, "Failed to publish command response to NATS");
            } else {
                tracing::info!("Command response relayed to NATS");
            }
        }
        tracing::info!("Response relay loop ended");
    });

    // Spawn telemetry publishing loop.
    let nats_tel = nats.clone();
    let vin = config.vin.clone();
    let telemetry_handle = tokio::spawn(async move {
        let mut state = TelemetryState::new(vin);

        while let Some(signal_update) = telemetry_rx.recv().await {
            // 04-REQ-8.1: Update telemetry state and publish if changed.
            if let Some(json) = state.update(signal_update) {
                if let Err(e) = nats_tel.publish_telemetry(&json).await {
                    tracing::error!(error = %e, "Failed to publish telemetry to NATS");
                } else {
                    tracing::info!("Telemetry message published to NATS");
                }
            }
        }
        tracing::info!("Telemetry publishing loop ended");
    });

    // Wait for all tasks to complete (they run until the service is stopped).
    let _ = tokio::join!(command_handle, response_handle, telemetry_handle);

    tracing::info!("Service shutting down");
    ExitCode::SUCCESS
}

/// Extract headers from a NATS message into a HashMap.
///
/// Converts `async_nats::HeaderMap` entries into a simple
/// `HashMap<String, String>` compatible with `validate_bearer_token`.
fn extract_headers(msg: &async_nats::Message) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    if let Some(nats_headers) = &msg.headers {
        for (key, values) in nats_headers.iter() {
            // Use the first value for each header key.
            if let Some(value) = values.iter().next() {
                headers.insert(key.to_string(), value.to_string());
            }
        }
    }
    headers
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        // Placeholder: verifies the binary crate compiles.
    }
}
