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
    _broker_addr: &str,
    _path: &str,
    _value: DatapointValue,
) -> Result<(), Box<dyn std::error::Error>> {
    todo!("publish_datapoint not yet implemented")
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
