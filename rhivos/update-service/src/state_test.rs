use super::AdapterState;

/// TS-07-P1: All valid transitions from 07-REQ-6.1 are accepted.
#[test]
fn test_valid_transitions() {
    let valid = vec![
        (AdapterState::Unknown, AdapterState::Downloading),
        (AdapterState::Downloading, AdapterState::Installing),
        (AdapterState::Downloading, AdapterState::Error),
        (AdapterState::Installing, AdapterState::Running),
        (AdapterState::Installing, AdapterState::Error),
        (AdapterState::Running, AdapterState::Stopped),
        (AdapterState::Running, AdapterState::Error),
        (AdapterState::Stopped, AdapterState::Running),
        (AdapterState::Stopped, AdapterState::Offloading),
        (AdapterState::Error, AdapterState::Downloading),
    ];

    for (from, to) in &valid {
        assert!(
            from.can_transition_to(*to),
            "expected valid transition {:?} -> {:?} to be accepted",
            from,
            to
        );
    }
}

/// TS-07-P1: Invalid transitions are rejected.
#[test]
fn test_invalid_transitions() {
    let invalid = vec![
        (AdapterState::Unknown, AdapterState::Running),
        (AdapterState::Unknown, AdapterState::Installing),
        (AdapterState::Unknown, AdapterState::Stopped),
        (AdapterState::Unknown, AdapterState::Error),
        (AdapterState::Unknown, AdapterState::Offloading),
        (AdapterState::Downloading, AdapterState::Running),
        (AdapterState::Downloading, AdapterState::Stopped),
        (AdapterState::Downloading, AdapterState::Offloading),
        (AdapterState::Downloading, AdapterState::Unknown),
        (AdapterState::Installing, AdapterState::Downloading),
        (AdapterState::Installing, AdapterState::Stopped),
        (AdapterState::Installing, AdapterState::Offloading),
        (AdapterState::Installing, AdapterState::Unknown),
        (AdapterState::Running, AdapterState::Downloading),
        (AdapterState::Running, AdapterState::Installing),
        (AdapterState::Running, AdapterState::Offloading),
        (AdapterState::Running, AdapterState::Unknown),
        (AdapterState::Stopped, AdapterState::Downloading),
        (AdapterState::Stopped, AdapterState::Installing),
        (AdapterState::Stopped, AdapterState::Error),
        (AdapterState::Stopped, AdapterState::Unknown),
        (AdapterState::Error, AdapterState::Running),
        (AdapterState::Error, AdapterState::Installing),
        (AdapterState::Error, AdapterState::Stopped),
        (AdapterState::Error, AdapterState::Offloading),
        (AdapterState::Error, AdapterState::Unknown),
        (AdapterState::Offloading, AdapterState::Unknown),
        (AdapterState::Offloading, AdapterState::Downloading),
        (AdapterState::Offloading, AdapterState::Running),
        (AdapterState::Offloading, AdapterState::Stopped),
        (AdapterState::Offloading, AdapterState::Error),
        (AdapterState::Offloading, AdapterState::Installing),
    ];

    for (from, to) in &invalid {
        assert!(
            !from.can_transition_to(*to),
            "expected invalid transition {:?} -> {:?} to be rejected",
            from,
            to
        );
    }
}

/// TS-07-P1: All 7 states exist in the enum.
#[test]
fn test_all_states_represented() {
    let all = AdapterState::all();
    assert_eq!(all.len(), 7, "expected exactly 7 adapter states");

    // Verify all expected states are present
    assert!(all.contains(&AdapterState::Unknown));
    assert!(all.contains(&AdapterState::Downloading));
    assert!(all.contains(&AdapterState::Installing));
    assert!(all.contains(&AdapterState::Running));
    assert!(all.contains(&AdapterState::Stopped));
    assert!(all.contains(&AdapterState::Error));
    assert!(all.contains(&AdapterState::Offloading));
}

/// Self-transitions should be rejected.
#[test]
fn test_self_transitions_rejected() {
    for state in AdapterState::all() {
        assert!(
            !state.can_transition_to(*state),
            "expected self-transition {:?} -> {:?} to be rejected",
            state,
            state
        );
    }
}
