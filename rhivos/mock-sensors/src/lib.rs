/// Generated code from vendored kuksa.val.v1 proto files.
pub mod kuksa {
    pub mod val {
        pub mod v1 {
            tonic::include_proto!("kuksa.val.v1");
        }
    }
}

use kuksa::val::v1::{
    datapoint::Value, val_client::ValClient, Datapoint, DataEntry, EntryUpdate, Field, SetRequest,
};

/// Value types that can be published to the DATA_BROKER.
pub enum DatapointValue {
    /// 64-bit floating point (e.g., latitude, longitude).
    Double(f64),
    /// 32-bit floating point (e.g., speed).
    Float(f32),
    /// Boolean (e.g., door open/closed).
    Bool(bool),
}

/// Publish a single VSS datapoint to DATA_BROKER via kuksa.val.v1 gRPC `Set` RPC.
///
/// # Arguments
///
/// * `broker_addr` - The DATA_BROKER gRPC endpoint (e.g., `http://localhost:55556`).
/// * `path` - The VSS signal path (e.g., `Vehicle.Speed`).
/// * `value` - The value to publish.
pub async fn publish_datapoint(
    broker_addr: &str,
    path: &str,
    value: DatapointValue,
) -> Result<(), Box<dyn std::error::Error>> {
    let dp_value = match value {
        DatapointValue::Double(v) => Value::DoubleValue(v),
        DatapointValue::Float(v) => Value::FloatValue(v),
        DatapointValue::Bool(v) => Value::BoolValue(v),
    };

    let mut client = ValClient::connect(broker_addr.to_string()).await?;

    let request = tonic::Request::new(SetRequest {
        updates: vec![EntryUpdate {
            entry: Some(DataEntry {
                path: path.to_string(),
                value: Some(Datapoint {
                    timestamp: 0,
                    value: Some(dp_value),
                }),
                actuator_target: None,
            }),
            fields: vec![Field::Value as i32],
        }],
    });

    client.set(request).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
