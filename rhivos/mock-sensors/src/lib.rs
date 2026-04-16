/// Value types that can be published to DATA_BROKER.
///
/// Task group 2 will wire these to kuksa.val.v1 proto types.
pub enum DatapointValue {
    Double(f64),
    Float(f32),
    Bool(bool),
}

/// Publish a single VSS signal to DATA_BROKER via kuksa.val.v1 gRPC Set RPC.
///
/// This is a stub — implementation comes in task group 2.
pub async fn publish_datapoint(
    _broker_addr: &str,
    _path: &str,
    _value: DatapointValue,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: implement gRPC client in task group 2
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
