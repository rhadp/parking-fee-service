use super::*;

/// TS-07-P1: All 7 adapter states are represented in the enum.
#[test]
fn test_all_states_represented() {
    let states = AdapterState::all_states();
    assert_eq!(states.len(), 7, "Expected exactly 7 adapter states");

    // Verify each expected state is present
    assert!(states.contains(&AdapterState::Unknown));
    assert!(states.contains(&AdapterState::Downloading));
    assert!(states.contains(&AdapterState::Installing));
    assert!(states.contains(&AdapterState::Running));
    assert!(states.contains(&AdapterState::Stopped));
    assert!(states.contains(&AdapterState::Error));
    assert!(states.contains(&AdapterState::Offloading));
}

/// TS-07-P1: All valid state transitions from 07-REQ-6.1 are accepted.
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
#[test]
fn test_valid_transitions() {
    let valid_transitions = vec![
        (AdapterState::Unknown, AdapterState::Downloading),
        (AdapterState::Downloading, AdapterState::Installing),
        (AdapterState::Downloading, AdapterState::Error),
        (AdapterState::Installing, AdapterState::Running),
        (AdapterState::Installing, AdapterState::Error),
        (AdapterState::Running, AdapterState::Stopped),
        (AdapterState::Running, AdapterState::Error),
        (AdapterState::Stopped, AdapterState::Running),
        (AdapterState::Stopped, AdapterState::Offloading),
    ];

    for (from, to) in &valid_transitions {
        assert!(
            from.can_transition_to(*to),
            "Expected valid transition {:?} -> {:?} to be accepted",
            from,
            to
        );
    }
}

/// TS-07-P1: All invalid state transitions are rejected.
/// Enumerate all (state, state) pairs and verify non-valid ones are rejected.
#[test]
fn test_invalid_transitions() {
    let valid_transitions: Vec<(AdapterState, AdapterState)> = vec![
        (AdapterState::Unknown, AdapterState::Downloading),
        (AdapterState::Downloading, AdapterState::Installing),
        (AdapterState::Downloading, AdapterState::Error),
        (AdapterState::Installing, AdapterState::Running),
        (AdapterState::Installing, AdapterState::Error),
        (AdapterState::Running, AdapterState::Stopped),
        (AdapterState::Running, AdapterState::Error),
        (AdapterState::Stopped, AdapterState::Running),
        (AdapterState::Stopped, AdapterState::Offloading),
    ];

    let all_states = AdapterState::all_states();

    for from in all_states {
        for to in all_states {
            let is_valid = valid_transitions.contains(&(*from, *to));
            if !is_valid {
                assert!(
                    !from.can_transition_to(*to),
                    "Expected invalid transition {:?} -> {:?} to be rejected",
                    from,
                    to
                );
            }
        }
    }
}
