/// Generated kuksa.val.v2 gRPC types and client.
pub mod kuksa {
    pub mod val {
        pub mod v2 {
            tonic::include_proto!("kuksa.val.v2");
        }
    }
}

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
    _broker_addr: &str,
    _path: &str,
    _value: DatapointValue,
) -> Result<(), Box<dyn std::error::Error>> {
    // Stub: will be implemented in task group 2
    todo!("publish_datapoint not yet implemented")
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
