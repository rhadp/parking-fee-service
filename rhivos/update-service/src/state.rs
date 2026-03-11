/// Adapter lifecycle states matching the protobuf AdapterState enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdapterState {
    Unknown,
    Downloading,
    Installing,
    Running,
    Stopped,
    Error,
    Offloading,
}

impl AdapterState {
    /// Returns true if transitioning from `self` to `target` is a valid
    /// state machine transition per 07-REQ-6.1.
    pub fn can_transition_to(&self, target: AdapterState) -> bool {
        use AdapterState::*;
        matches!(
            (self, target),
            (Unknown, Downloading)
                | (Downloading, Installing)
                | (Downloading, Error)
                | (Installing, Running)
                | (Installing, Error)
                | (Running, Stopped)
                | (Running, Error)
                | (Stopped, Running)
                | (Stopped, Offloading)
        )
    }

    /// Returns all defined adapter states.
    pub fn all_states() -> &'static [AdapterState] {
        &[
            AdapterState::Unknown,
            AdapterState::Downloading,
            AdapterState::Installing,
            AdapterState::Running,
            AdapterState::Stopped,
            AdapterState::Error,
            AdapterState::Offloading,
        ]
    }
}

#[cfg(test)]
#[path = "state_test.rs"]
mod state_test;
