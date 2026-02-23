//! Adapter lifecycle state machine and manager.
//!
//! Stub module — implementation will be added in task group 5.

use std::fmt;

/// Adapter lifecycle states matching the proto enum values.
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

impl fmt::Display for AdapterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdapterState::Unknown => write!(f, "UNKNOWN"),
            AdapterState::Downloading => write!(f, "DOWNLOADING"),
            AdapterState::Installing => write!(f, "INSTALLING"),
            AdapterState::Running => write!(f, "RUNNING"),
            AdapterState::Stopped => write!(f, "STOPPED"),
            AdapterState::Error => write!(f, "ERROR"),
            AdapterState::Offloading => write!(f, "OFFLOADING"),
        }
    }
}

/// Error returned when an invalid state transition is attempted.
#[derive(Debug, Clone, PartialEq)]
pub struct InvalidTransition {
    pub from: AdapterState,
    pub to: AdapterState,
}

impl fmt::Display for InvalidTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid state transition: {} -> {}",
            self.from, self.to
        )
    }
}

impl std::error::Error for InvalidTransition {}

/// Check whether a state transition is valid per 04-REQ-7.1.
///
/// Valid transitions:
/// - UNKNOWN -> DOWNLOADING
/// - DOWNLOADING -> INSTALLING
/// - DOWNLOADING -> ERROR
/// - INSTALLING -> RUNNING
/// - INSTALLING -> ERROR
/// - RUNNING -> STOPPED
/// - STOPPED -> OFFLOADING
/// - STOPPED -> DOWNLOADING (re-install)
/// - OFFLOADING -> UNKNOWN (removed)
/// - ERROR -> DOWNLOADING (retry)
pub fn is_valid_transition(from: AdapterState, to: AdapterState) -> bool {
    // Stub: always returns false — real implementation in task group 5
    let _ = (from, to);
    false
}

/// Attempt a state transition. Returns Ok(new_state) if valid,
/// Err(InvalidTransition) if not.
pub fn try_transition(
    from: AdapterState,
    to: AdapterState,
) -> Result<AdapterState, InvalidTransition> {
    if is_valid_transition(from, to) {
        Ok(to)
    } else {
        Err(InvalidTransition { from, to })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // TS-04-27: Valid state transitions enforced
    // Requirement: 04-REQ-7.1
    // -----------------------------------------------------------------------

    #[test]
    fn test_valid_state_transitions() {
        let valid_transitions = vec![
            (AdapterState::Unknown, AdapterState::Downloading),
            (AdapterState::Downloading, AdapterState::Installing),
            (AdapterState::Downloading, AdapterState::Error),
            (AdapterState::Installing, AdapterState::Running),
            (AdapterState::Installing, AdapterState::Error),
            (AdapterState::Running, AdapterState::Stopped),
            (AdapterState::Stopped, AdapterState::Offloading),
            (AdapterState::Stopped, AdapterState::Downloading),
            (AdapterState::Offloading, AdapterState::Unknown),
            (AdapterState::Error, AdapterState::Downloading),
        ];

        for (from, to) in valid_transitions {
            assert!(
                is_valid_transition(from, to),
                "expected valid transition: {} -> {}",
                from,
                to,
            );
            assert!(
                try_transition(from, to).is_ok(),
                "try_transition should succeed for {} -> {}",
                from,
                to,
            );
        }
    }

    // -----------------------------------------------------------------------
    // TS-04-28: Invalid state transitions rejected
    // Requirement: 04-REQ-7.2
    // -----------------------------------------------------------------------

    #[test]
    fn test_invalid_state_transitions() {
        let invalid_transitions = vec![
            (AdapterState::Unknown, AdapterState::Running),
            (AdapterState::Unknown, AdapterState::Installing),
            (AdapterState::Unknown, AdapterState::Stopped),
            (AdapterState::Downloading, AdapterState::Stopped),
            (AdapterState::Downloading, AdapterState::Running),
            (AdapterState::Installing, AdapterState::Downloading),
            (AdapterState::Installing, AdapterState::Stopped),
            (AdapterState::Running, AdapterState::Downloading),
            (AdapterState::Running, AdapterState::Installing),
            (AdapterState::Offloading, AdapterState::Running),
            (AdapterState::Offloading, AdapterState::Stopped),
            (AdapterState::Error, AdapterState::Running),
            (AdapterState::Error, AdapterState::Installing),
        ];

        for (from, to) in invalid_transitions {
            assert!(
                !is_valid_transition(from, to),
                "expected invalid transition: {} -> {}",
                from,
                to,
            );
            assert!(
                try_transition(from, to).is_err(),
                "try_transition should fail for {} -> {}",
                from,
                to,
            );
        }
    }

    // -----------------------------------------------------------------------
    // TS-04-P4: State Machine Integrity (property test)
    // Property: For any (S, T) not in valid transitions, transition is rejected.
    // Validates: 04-REQ-7.1, 04-REQ-7.2
    // -----------------------------------------------------------------------

    #[test]
    fn test_property_state_machine_integrity() {
        use std::collections::HashSet;

        let all_states = vec![
            AdapterState::Unknown,
            AdapterState::Downloading,
            AdapterState::Installing,
            AdapterState::Running,
            AdapterState::Stopped,
            AdapterState::Offloading,
            AdapterState::Error,
        ];

        let valid: HashSet<(AdapterState, AdapterState)> = [
            (AdapterState::Unknown, AdapterState::Downloading),
            (AdapterState::Downloading, AdapterState::Installing),
            (AdapterState::Downloading, AdapterState::Error),
            (AdapterState::Installing, AdapterState::Running),
            (AdapterState::Installing, AdapterState::Error),
            (AdapterState::Running, AdapterState::Stopped),
            (AdapterState::Stopped, AdapterState::Offloading),
            (AdapterState::Stopped, AdapterState::Downloading),
            (AdapterState::Offloading, AdapterState::Unknown),
            (AdapterState::Error, AdapterState::Downloading),
        ]
        .into_iter()
        .collect();

        // Exhaustive check: for every (from, to) pair, transition validity
        // must match whether the pair is in the valid set.
        for from in &all_states {
            for to in &all_states {
                let expected_valid = valid.contains(&(*from, *to));
                let actual_valid = is_valid_transition(*from, *to);
                assert_eq!(
                    actual_valid, expected_valid,
                    "state machine integrity: {} -> {} should be {}",
                    from,
                    to,
                    if expected_valid { "valid" } else { "invalid" },
                );
            }
        }
    }
}
