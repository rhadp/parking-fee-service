//! Property-based tests for UPDATE_SERVICE (TS-07-P1 through TS-07-P6).

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::adapter::{derive_adapter_id, AdapterState, AdapterStateEvent};
    use crate::grpc::proto::update_service_server::UpdateService;
    use crate::state::StateManager;

    // TS-07-P1: Adapter ID Determinism
    // For any valid OCI image reference, derive_adapter_id returns the same
    // result on every call, and different last-segment:tag combinations
    // produce different IDs.
    #[test]
    #[ignore]
    fn proptest_adapter_id_determinism() {
        proptest!(|(
            registry in "[a-z0-9]+(\\.[a-z0-9]+)*/",
            name in "[a-z][a-z0-9\\-]{0,20}",
            tag in "[a-z0-9][a-z0-9\\.\\-]{0,10}",
        )| {
            let image_ref = format!("{registry}{name}:{tag}");
            let id1 = derive_adapter_id(&image_ref);
            let id2 = derive_adapter_id(&image_ref);
            prop_assert_eq!(&id1, &id2, "same input must produce same output");
        });
    }

    // TS-07-P1 (uniqueness): Different name:tag → different adapter IDs
    #[test]
    #[ignore]
    fn proptest_adapter_id_uniqueness() {
        proptest!(|(
            registry in "[a-z0-9]+\\.example\\.com/",
            name_a in "[a-z][a-z0-9]{1,10}",
            tag_a in "v[0-9]{1,3}",
            name_b in "[a-z][a-z0-9]{1,10}",
            tag_b in "v[0-9]{1,3}",
        )| {
            let seg_a = format!("{name_a}:{tag_a}");
            let seg_b = format!("{name_b}:{tag_b}");
            if seg_a != seg_b {
                let ref_a = format!("{registry}{name_a}:{tag_a}");
                let ref_b = format!("{registry}{name_b}:{tag_b}");
                let id_a = derive_adapter_id(&ref_a);
                let id_b = derive_adapter_id(&ref_b);
                prop_assert_ne!(id_a, id_b, "different name:tag must produce different IDs");
            }
        });
    }

    // TS-07-P2: Single Adapter Invariant
    // At most one adapter is in RUNNING state at any time.
    // NOTE: This property test requires the install orchestration to be
    // implemented. For now it tests the state manager directly.
    #[test]
    #[ignore]
    fn proptest_single_adapter_invariant() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        proptest!(|(n in 1_usize..5)| {
            rt.block_on(async {
                let (tx, _rx) = tokio::sync::broadcast::channel(64);
                let mgr = StateManager::new(tx);
                for i in 0..n {
                    let entry = crate::adapter::AdapterEntry {
                        adapter_id: format!("adapter-{i}"),
                        image_ref: format!("example.com/adapter-{i}:v1"),
                        checksum_sha256: format!("sha256:check{i}"),
                        state: AdapterState::Unknown,
                        job_id: format!("job-{i}"),
                        stopped_at: None,
                        error_message: None,
                    };
                    // Stop any currently running adapter
                    if let Some(running) = mgr.get_running_adapter() {
                        mgr.transition(&running.adapter_id, AdapterState::Stopped, None).unwrap();
                    }
                    mgr.create_adapter(entry);
                    mgr.transition(&format!("adapter-{i}"), AdapterState::Downloading, None).unwrap();
                    mgr.transition(&format!("adapter-{i}"), AdapterState::Installing, None).unwrap();
                    mgr.transition(&format!("adapter-{i}"), AdapterState::Running, None).unwrap();

                    let running_count = mgr
                        .list_adapters()
                        .iter()
                        .filter(|a| a.state == AdapterState::Running)
                        .count();
                    prop_assert!(
                        running_count <= 1,
                        "at most 1 adapter should be RUNNING, got {}",
                        running_count
                    );
                }
                Ok::<(), TestCaseError>(())
            })?;
        });
    }

    // TS-07-P3: State Transition Validity
    // Every observed state transition follows the valid state machine edges.
    #[test]
    #[ignore]
    fn proptest_state_transition_validity() {
        use std::collections::HashSet;

        let valid_transitions: HashSet<(AdapterState, AdapterState)> = [
            (AdapterState::Unknown, AdapterState::Downloading),
            (AdapterState::Downloading, AdapterState::Installing),
            (AdapterState::Downloading, AdapterState::Error),
            (AdapterState::Installing, AdapterState::Running),
            (AdapterState::Installing, AdapterState::Error),
            (AdapterState::Running, AdapterState::Stopped),
            (AdapterState::Running, AdapterState::Error),
            (AdapterState::Stopped, AdapterState::Offloading),
            (AdapterState::Stopped, AdapterState::Error),
            (AdapterState::Offloading, AdapterState::Error),
        ]
        .into_iter()
        .collect();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        proptest!(|(transitions in prop::collection::vec(0_u8..7, 1..6))| {
            rt.block_on(async {
                let (tx, mut rx) = tokio::sync::broadcast::channel(64);
                let mgr = StateManager::new(tx);
                let entry = crate::adapter::AdapterEntry {
                    adapter_id: "test-adapter".to_string(),
                    image_ref: "example.com/test:v1".to_string(),
                    checksum_sha256: "sha256:test".to_string(),
                    state: AdapterState::Unknown,
                    job_id: "test-job".to_string(),
                    stopped_at: None,
                    error_message: None,
                };
                mgr.create_adapter(entry);

                let states = [
                    AdapterState::Unknown,
                    AdapterState::Downloading,
                    AdapterState::Installing,
                    AdapterState::Running,
                    AdapterState::Stopped,
                    AdapterState::Error,
                    AdapterState::Offloading,
                ];

                for &t in &transitions {
                    let target = states[t as usize % states.len()].clone();
                    let _ = mgr.transition("test-adapter", target, None);
                }

                // Drain all events and check validity
                while let Ok(event) = rx.try_recv() {
                    prop_assert!(
                        valid_transitions.contains(&(event.old_state.clone(), event.new_state.clone())),
                        "invalid transition: {:?} -> {:?}",
                        event.old_state,
                        event.new_state
                    );
                }

                Ok::<(), TestCaseError>(())
            })?;
        });
    }

    // TS-07-P4: Event Delivery Completeness
    // All active subscribers receive the same state events.
    #[test]
    #[ignore]
    fn proptest_event_delivery_completeness() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        proptest!(|(n_subscribers in 1_usize..4)| {
            rt.block_on(async {
                let (tx, _) = tokio::sync::broadcast::channel(64);
                let mgr = StateManager::new(tx.clone());

                // Create subscribers
                let mut receivers: Vec<_> = (0..n_subscribers)
                    .map(|_| tx.subscribe())
                    .collect();

                let entry = crate::adapter::AdapterEntry {
                    adapter_id: "test-adapter".to_string(),
                    image_ref: "example.com/test:v1".to_string(),
                    checksum_sha256: "sha256:test".to_string(),
                    state: AdapterState::Unknown,
                    job_id: "test-job".to_string(),
                    stopped_at: None,
                    error_message: None,
                };
                mgr.create_adapter(entry);
                mgr.transition("test-adapter", AdapterState::Downloading, None).unwrap();
                mgr.transition("test-adapter", AdapterState::Installing, None).unwrap();
                mgr.transition("test-adapter", AdapterState::Running, None).unwrap();

                // Collect events from each subscriber
                let mut all_events: Vec<Vec<AdapterStateEvent>> = Vec::new();
                for rx in &mut receivers {
                    let mut events = Vec::new();
                    while let Ok(event) = rx.try_recv() {
                        events.push(event);
                    }
                    all_events.push(events);
                }

                // All subscribers should have identical events
                for i in 1..all_events.len() {
                    prop_assert_eq!(
                        &all_events[0],
                        &all_events[i],
                        "subscriber {} events differ from subscriber 0",
                        i
                    );
                }

                Ok::<(), TestCaseError>(())
            })?;
        });
    }

    // TS-07-P5: Checksum Verification Soundness
    // When digest != checksum, adapter transitions to ERROR and image is removed.
    // NOTE: Requires install workflow implementation. Tests the principle
    // via state manager and mock podman.
    #[test]
    #[ignore]
    fn proptest_checksum_verification_soundness() {
        use crate::podman::mock::MockPodmanExecutor;

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        proptest!(|(
            digest_suffix in "[a-f0-9]{8,16}",
            checksum_suffix in "[a-f0-9]{8,16}",
        )| {
            let digest = format!("sha256:{digest_suffix}");
            let checksum = format!("sha256:{checksum_suffix}");
            if digest != checksum {
                rt.block_on(async {
                    let mock = std::sync::Arc::new(MockPodmanExecutor::new());
                    mock.set_pull_result(Ok(()));
                    mock.set_inspect_result(Ok(digest.clone()));

                    let (tx, _rx) = tokio::sync::broadcast::channel(64);
                    let state_mgr = std::sync::Arc::new(StateManager::new(tx.clone()));
                    let svc = crate::grpc::UpdateServiceImpl::new(
                        state_mgr.clone(),
                        mock.clone(),
                        tx,
                    );

                    let request = tonic::Request::new(crate::grpc::proto::InstallAdapterRequest {
                        image_ref: "example.com/test:v1".to_string(),
                        checksum_sha256: checksum,
                    });

                    let _ = svc.install_adapter(request).await;
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                    let adapter = state_mgr.get_adapter("test-v1");
                    prop_assert!(adapter.is_some(), "adapter should exist");
                    prop_assert_eq!(
                        adapter.unwrap().state,
                        AdapterState::Error,
                        "adapter should be ERROR on checksum mismatch"
                    );
                    prop_assert!(
                        mock.rmi_calls().contains(&"example.com/test:v1".to_string()),
                        "rmi should have been called"
                    );

                    Ok::<(), TestCaseError>(())
                })?;
            }
        });
    }

    // TS-07-P6: Offload Timing Correctness
    // Offloading does not occur before the inactivity timeout has elapsed.
    // Verifies both the immediate-check (S=0) and the boundary condition
    // (S = T - 1s) where T is the inactivity timeout.
    #[test]
    #[ignore]
    fn proptest_offload_timing_correctness() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        proptest!(|(timeout_secs in 2_u64..5)| {
            rt.block_on(async {
                let (tx, _rx) = tokio::sync::broadcast::channel(64);
                let state_mgr = std::sync::Arc::new(StateManager::new(tx));

                let entry = crate::adapter::AdapterEntry {
                    adapter_id: "offload-test".to_string(),
                    image_ref: "example.com/test:v1".to_string(),
                    checksum_sha256: "sha256:test".to_string(),
                    state: AdapterState::Unknown,
                    job_id: "test-job".to_string(),
                    stopped_at: None,
                    error_message: None,
                };
                state_mgr.create_adapter(entry);
                state_mgr.transition("offload-test", AdapterState::Downloading, None).unwrap();
                state_mgr.transition("offload-test", AdapterState::Installing, None).unwrap();
                state_mgr.transition("offload-test", AdapterState::Running, None).unwrap();
                state_mgr.transition("offload-test", AdapterState::Stopped, None).unwrap();

                let timeout = std::time::Duration::from_secs(timeout_secs);

                // Check 1: immediately after stopping (S ≈ 0), no offload
                let candidates = state_mgr.get_offload_candidates(timeout);
                prop_assert!(
                    candidates.is_empty(),
                    "should have no offload candidates immediately after stopping"
                );

                // Check 2: boundary condition (S = T - 1s), still no offload.
                // Backdate stopped_at to (T - 1s) ago, which is just
                // before the timeout expires.
                let just_before = std::time::Instant::now()
                    - timeout
                    + std::time::Duration::from_secs(1);
                state_mgr.set_stopped_at("offload-test", just_before);
                let candidates = state_mgr.get_offload_candidates(timeout);
                prop_assert!(
                    candidates.is_empty(),
                    "should have no offload candidates 1s before timeout expires"
                );

                // Check 3: after timeout expires, adapter IS a candidate.
                // Backdate stopped_at to (T + 1s) ago.
                let well_past = std::time::Instant::now()
                    - timeout
                    - std::time::Duration::from_secs(1);
                state_mgr.set_stopped_at("offload-test", well_past);
                let candidates = state_mgr.get_offload_candidates(timeout);
                prop_assert_eq!(
                    candidates.len(),
                    1,
                    "should have 1 offload candidate after timeout"
                );
                prop_assert_eq!(
                    &candidates[0].adapter_id,
                    "offload-test",
                    "offload candidate should be the correct adapter"
                );

                // Adapter should still exist in state
                let adapter = state_mgr.get_adapter("offload-test");
                prop_assert!(adapter.is_some(), "adapter should still exist (not removed yet)");
                prop_assert_eq!(
                    adapter.unwrap().state,
                    AdapterState::Stopped,
                    "adapter should still be STOPPED"
                );

                Ok::<(), TestCaseError>(())
            })?;
        });
    }
}
