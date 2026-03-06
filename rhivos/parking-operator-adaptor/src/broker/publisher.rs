/// DATA_BROKER publisher for SessionActive signal.
/// Stub: will be implemented in task group 3.
pub struct BrokerPublisher {
    _addr: String,
}

impl BrokerPublisher {
    pub fn new(addr: &str) -> Self {
        Self {
            _addr: addr.to_string(),
        }
    }
}
