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
    ///
    /// Valid transitions:
    ///   UNKNOWN -> DOWNLOADING
    ///   DOWNLOADING -> INSTALLING
    ///   DOWNLOADING -> ERROR
    ///   INSTALLING -> RUNNING
    ///   INSTALLING -> ERROR
    ///   RUNNING -> STOPPED
    ///   RUNNING -> ERROR
    ///   STOPPED -> RUNNING
    ///   STOPPED -> OFFLOADING
    pub fn can_transition_to(&self, target: AdapterState) -> bool {
        matches!(
            (self, target),
            (AdapterState::Unknown, AdapterState::Downloading)
                | (AdapterState::Downloading, AdapterState::Installing)
                | (AdapterState::Downloading, AdapterState::Error)
                | (AdapterState::Installing, AdapterState::Running)
                | (AdapterState::Installing, AdapterState::Error)
                | (AdapterState::Running, AdapterState::Stopped)
                | (AdapterState::Running, AdapterState::Error)
                | (AdapterState::Stopped, AdapterState::Running)
                | (AdapterState::Stopped, AdapterState::Offloading)
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
