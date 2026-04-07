/// Generated kuksa.val.v2 gRPC types and client.
pub mod kuksa {
    pub mod val {
        pub mod v2 {
            tonic::include_proto!("kuksa.val.v2");
        }
    }
}

use kuksa::val::v2::{
    val_client::ValClient, Datapoint, PublishValueRequest, SignalId, Value,
};

/// Datapoint value types for publishing to DATA_BROKER.
pub enum DatapointValue {
    Double(f64),
    Float(f32),
    Bool(bool),
}

/// Publishes a single VSS datapoint to DATA_BROKER via kuksa.val.v2 gRPC PublishValue RPC.
///
/// # Arguments
/// * `broker_addr` - The address of the DATA_BROKER (e.g., "http://localhost:55556").
/// * `path` - The VSS signal path (e.g., "Vehicle.CurrentLocation.Latitude").
/// * `value` - The value to publish.
///
/// # Errors
/// Returns an error if the connection fails or the PublishValue RPC returns an error.
pub async fn publish_datapoint(
    broker_addr: &str,
    path: &str,
    value: DatapointValue,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ValClient::connect(broker_addr.to_string()).await?;

    let typed_value = match value {
        DatapointValue::Double(v) => Value {
            typed_value: Some(kuksa::val::v2::value::TypedValue::Double(v)),
        },
        DatapointValue::Float(v) => Value {
            typed_value: Some(kuksa::val::v2::value::TypedValue::Float(v)),
        },
        DatapointValue::Bool(v) => Value {
            typed_value: Some(kuksa::val::v2::value::TypedValue::Bool(v)),
        },
    };

    let request = PublishValueRequest {
        signal_id: Some(SignalId {
            signal: Some(kuksa::val::v2::signal_id::Signal::Path(path.to_string())),
        }),
        data_point: Some(Datapoint {
            timestamp: None,
            value: Some(typed_value),
        }),
    };

    client.publish_value(request).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datapoint_value_variants() {
        // Verify enum variants can be constructed
        let _ = DatapointValue::Double(48.1351);
        let _ = DatapointValue::Float(60.0);
        let _ = DatapointValue::Bool(true);
    }
}
