/// DATA_BROKER subscriber for lock/unlock events.
/// Stub: will be implemented in task group 3.
pub struct BrokerSubscriber {
    _addr: String,
}

impl BrokerSubscriber {
    pub fn new(addr: &str) -> Self {
        Self {
            _addr: addr.to_string(),
        }
    }
}
