//! Shared library for mock sensor binaries.
//!
//! Provides `publish_datapoint`, which connects to DATA_BROKER and sets a
//! single VSS signal value via the kuksa.val.v1 `Set` RPC.

/// Generated types from `proto/kuksa/val.proto`.
///
/// The `enum_variant_names` lint is suppressed because the generated `Value`
/// enum has variants like `StringValue`, `BoolValue`, etc. — a protobuf
/// convention that clippy flags but we cannot change in generated code.
#[allow(clippy::enum_variant_names)]
pub mod kuksa {
    tonic::include_proto!("kuksa");
}

use kuksa::val_service_client::ValServiceClient;
use kuksa::{DataEntry, Datapoint, SetRequest};

/// Value types that can be published to DATA_BROKER.
pub enum DatapointValue {
    Double(f64),
    Float(f32),
    Bool(bool),
}

/// Publish a single VSS signal to DATA_BROKER via kuksa.val.v1 gRPC `Set` RPC.
///
/// Connects to `broker_addr`, sets `path` to `value`, and returns.
/// Returns `Err` if the connection fails or the broker reports an error.
pub async fn publish_datapoint(
    broker_addr: &str,
    path: &str,
    value: DatapointValue,
) -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = tonic::transport::Endpoint::from_shared(broker_addr.to_owned())
        .map_err(|e| format!("Invalid broker address '{broker_addr}': {e}"))?
        .connect_timeout(std::time::Duration::from_secs(5));

    let channel = endpoint
        .connect()
        .await
        .map_err(|e| format!("Failed to connect to DATA_BROKER at {broker_addr}: {e}"))?;

    let mut client = ValServiceClient::new(channel);

    let datapoint_value = match value {
        DatapointValue::Double(v) => kuksa::datapoint::Value::DoubleValue(v),
        DatapointValue::Float(v) => kuksa::datapoint::Value::FloatValue(v),
        DatapointValue::Bool(v) => kuksa::datapoint::Value::BoolValue(v),
    };

    let request = SetRequest {
        updates: vec![DataEntry {
            path: path.to_owned(),
            value: Some(Datapoint {
                timestamp: 0,
                value: Some(datapoint_value),
            }),
        }],
    };

    let response = client
        .set(request)
        .await
        .map_err(|e| format!("Set RPC failed for '{path}': {e}"))?
        .into_inner();

    if !response.errors.is_empty() {
        let msg = response
            .errors
            .iter()
            .map(|e| format!("[{}] {}: {}", e.code, e.reason, e.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("DATA_BROKER returned errors for '{path}': {msg}").into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_compiles() {
        // Ensure DatapointValue variants are constructible.
        let _d = DatapointValue::Double(1.0);
        let _f = DatapointValue::Float(0.5);
        let _b = DatapointValue::Bool(true);
        assert!(true);
    }
}
