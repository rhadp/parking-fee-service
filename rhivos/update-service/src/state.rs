/// Adapter lifecycle states.
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
    /// Returns all defined adapter states.
    pub fn all() -> &'static [AdapterState] {
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

    /// Check whether transitioning from `self` to `target` is valid.
    ///
    /// Valid transitions (07-REQ-6.1):
    /// UNKNOWN->DOWNLOADING, DOWNLOADING->INSTALLING, DOWNLOADING->ERROR,
    /// INSTALLING->RUNNING, INSTALLING->ERROR, RUNNING->STOPPED,
    /// RUNNING->ERROR, STOPPED->RUNNING, STOPPED->OFFLOADING,
    /// ERROR->DOWNLOADING
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
                | (AdapterState::Error, AdapterState::Downloading)
        )
    }

    /// Convert from proto i32 value to domain AdapterState.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(AdapterState::Unknown),
            1 => Some(AdapterState::Downloading),
            2 => Some(AdapterState::Installing),
            3 => Some(AdapterState::Running),
            4 => Some(AdapterState::Stopped),
            5 => Some(AdapterState::Error),
            6 => Some(AdapterState::Offloading),
            _ => None,
        }
    }
}

impl From<AdapterState> for i32 {
    fn from(state: AdapterState) -> i32 {
        match state {
            AdapterState::Unknown => 0,
            AdapterState::Downloading => 1,
            AdapterState::Installing => 2,
            AdapterState::Running => 3,
            AdapterState::Stopped => 4,
            AdapterState::Error => 5,
            AdapterState::Offloading => 6,
        }
    }
}

#[cfg(test)]
#[path = "state_test.rs"]
mod tests;
