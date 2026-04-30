use proptest::prelude::*;

use crate::adapter::{derive_adapter_id, AdapterState, AdapterStateEvent};
use crate::install::install_adapter;
use crate::podman::testing::MockPodmanExecutor;
use crate::state::StateManager;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Valid state transitions per the state machine in design.md.
const VALID_TRANSITIONS: &[(AdapterState, AdapterState)] = &[
    (AdapterState::Unknown, AdapterState::Downloading),
    (AdapterState::Downloading, AdapterState::Installing),
    (AdapterState::Downloading, AdapterState::Error),
    (AdapterState::Installing, AdapterState::Running),
    (AdapterState::Installing, AdapterState::Error),
    (AdapterState::Running, AdapterState::Stopped),
    (AdapterState::Running, AdapterState::Error),
    (AdapterState::Stopped, AdapterState::Offloading),
    (AdapterState::Offloading, AdapterState::Error),
];

proptest! {
    // TS-07-P1: Adapter ID Determinism
    // Property 1 from design.md — Validates 07-REQ-1.6
    // For any valid OCI image reference, derive_adapter_id returns the
    // same result on every call, and different last-segment:tag
    // combinations produce different IDs.
    #[test]
    #[ignore]
    fn proptest_adapter_id_determinism(
        registry in "[a-z0-9.-]{1,20}",
        path in "[a-z0-9/-]{0,30}",
        name in "[a-z][a-z0-9-]{0,15}",
        tag in "[a-z0-9][a-z0-9.-]{0,10}",
    ) {
        let image_ref = if path.is_empty() {
            format!("{registry}/{name}:{tag}")
        } else {
            format!("{registry}/{path}/{name}:{tag}")
        };
        let id1 = derive_adapter_id(&image_ref);
        let id2 = derive_adapter_id(&image_ref);
        prop_assert_eq!(&id1, &id2, "same input must produce same output");
        prop_assert!(!id1.is_empty(), "adapter ID must not be empty");
    }

    // TS-07-P1 (uniqueness): Adapter ID Uniqueness
    // Property 1 from design.md — Validates 07-REQ-1.6
    // Different last-segment:tag combinations SHALL produce different IDs.
    #[test]
    #[ignore]
    fn proptest_adapter_id_uniqueness(
        name1 in "[a-z][a-z0-9-]{0,15}",
        tag1 in "[a-z0-9][a-z0-9.-]{0,10}",
        name2 in "[a-z][a-z0-9-]{0,15}",
        tag2 in "[a-z0-9][a-z0-9.-]{0,10}",
    ) {
        let seg1 = format!("{name1}:{tag1}");
        let seg2 = format!("{name2}:{tag2}");
        prop_assume!(seg1 != seg2, "skip when last segments are identical");
        let ref1 = format!("registry.example.com/{name1}:{tag1}");
        let ref2 = format!("registry.example.com/{name2}:{tag2}");
        let id1 = derive_adapter_id(&ref1);
        let id2 = derive_adapter_id(&ref2);
        let msg = format!(
            "different name:tag should produce different IDs: {} vs {}",
            seg1, seg2
        );
        prop_assert_ne!(id1, id2, "{}", msg);
    }

    // TS-07-P2: Single Adapter Invariant
    // Property 2 from design.md — Validates 07-REQ-2.1, 07-REQ-2.2
    // At most one adapter is in RUNNING state at any time.
    #[test]
    #[ignore]
    fn proptest_single_adapter_invariant(n in 1usize..4) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let (tx, _rx) = broadcast::channel(128);
            let state_mgr = Arc::new(StateManager::new(tx));
            let mock_podman = Arc::new(MockPodmanExecutor::new());
            mock_podman.set_pull_result(Ok(()));
            mock_podman.set_run_result(Ok(()));
            mock_podman.set_stop_result(Ok(()));

            for i in 0..n {
                let checksum = format!("sha256:check{i}");
                let image_ref = format!("example.com/adapter-{i}:v1");
                mock_podman.set_inspect_result(Ok(checksum.clone()));
                install_adapter(
                    &image_ref,
                    &checksum,
                    state_mgr.clone(),
                    mock_podman.clone(),
                )
                .await
                .unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                let running_count = state_mgr
                    .list_adapters()
                    .iter()
                    .filter(|a| a.state == AdapterState::Running)
                    .count();
                prop_assert!(
                    running_count <= 1,
                    "at most 1 adapter should be RUNNING, got {running_count}"
                );
            }
            Ok(())
        })?;
    }

    // TS-07-P3: State Transition Validity
    // Property 3 from design.md — Validates 07-REQ-8.1
    // Every observed state transition follows valid state machine edges.
    #[test]
    #[ignore]
    fn proptest_state_transition_validity(dummy in 0u8..1) {
        let _ = dummy;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let (tx, mut rx) = broadcast::channel(128);
            let state_mgr = Arc::new(StateManager::new(tx));
            let mock_podman = Arc::new(MockPodmanExecutor::new());
            mock_podman.set_pull_result(Ok(()));
            mock_podman.set_inspect_result(Ok("sha256:abc".to_string()));
            mock_podman.set_run_result(Ok(()));

            install_adapter(
                "example.com/val-test:v1",
                "sha256:abc",
                state_mgr.clone(),
                mock_podman.clone(),
            )
            .await
            .unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            // Collect all events.
            let mut events = Vec::new();
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }

            for event in &events {
                let pair = (event.old_state.clone(), event.new_state.clone());
                prop_assert!(
                    VALID_TRANSITIONS.contains(&pair),
                    "invalid transition: {:?} -> {:?}",
                    event.old_state,
                    event.new_state
                );
            }
            Ok(())
        })?;
    }

    // TS-07-P4: Event Delivery Completeness
    // Property 4 from design.md — Validates 07-REQ-3.3, 07-REQ-8.3
    // All active subscribers receive identical event sequences.
    #[test]
    #[ignore]
    fn proptest_event_delivery_completeness(n_subscribers in 1usize..4) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let (tx, _initial_rx) = broadcast::channel(128);
            // Create N subscribers.
            let mut receivers: Vec<broadcast::Receiver<AdapterStateEvent>> = Vec::new();
            for _ in 0..n_subscribers {
                receivers.push(tx.subscribe());
            }

            let state_mgr = Arc::new(StateManager::new(tx));
            let mock_podman = Arc::new(MockPodmanExecutor::new());
            mock_podman.set_pull_result(Ok(()));
            mock_podman.set_inspect_result(Ok("sha256:abc".to_string()));
            mock_podman.set_run_result(Ok(()));

            install_adapter(
                "example.com/delivery-test:v1",
                "sha256:abc",
                state_mgr.clone(),
                mock_podman.clone(),
            )
            .await
            .unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            // Collect events from each subscriber.
            let mut all_events: Vec<Vec<AdapterStateEvent>> = Vec::new();
            for rx in &mut receivers {
                let mut events = Vec::new();
                while let Ok(event) = rx.try_recv() {
                    events.push(event);
                }
                all_events.push(events);
            }

            // All subscribers should have the same events.
            for i in 1..n_subscribers {
                prop_assert_eq!(
                    &all_events[0],
                    &all_events[i],
                    "subscriber 0 and {} should have identical events", i
                );
            }
            Ok(())
        })?;
    }

    // TS-07-P5: Checksum Verification Soundness
    // Property 5 from design.md — Validates 07-REQ-1.3, 07-REQ-1.E4
    // Mismatched checksum always leads to ERROR and image removal.
    #[test]
    #[ignore]
    fn proptest_checksum_verification_soundness(
        suffix in "[a-f0-9]{8,16}",
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let digest = format!("sha256:actual_{suffix}");
            let checksum = format!("sha256:expected_{suffix}");
            // Ensure they differ.
            prop_assert_ne!(&digest, &checksum);

            let (tx, _rx) = broadcast::channel(128);
            let state_mgr = Arc::new(StateManager::new(tx));
            let mock_podman = Arc::new(MockPodmanExecutor::new());
            mock_podman.set_pull_result(Ok(()));
            mock_podman.set_inspect_result(Ok(digest));
            mock_podman.set_rmi_result(Ok(()));

            let image_ref = "example.com/cs-test:v1";
            install_adapter(image_ref, &checksum, state_mgr.clone(), mock_podman.clone())
                .await
                .unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            let adapter = state_mgr
                .get_adapter("cs-test-v1")
                .expect("adapter should exist");
            prop_assert_eq!(adapter.state, AdapterState::Error);
            prop_assert!(
                mock_podman.rmi_calls().contains(&image_ref.to_string()),
                "rmi should have been called"
            );
            Ok(())
        })?;
    }

    // TS-07-P6: Offload Timing Correctness
    // Property 6 from design.md — Validates 07-REQ-6.1
    // Offloading does not occur before the inactivity timeout elapses.
    #[test]
    #[ignore]
    fn proptest_offload_timing_correctness(timeout_secs in 2u64..5) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let (tx, _rx) = broadcast::channel(128);
            let state_mgr = Arc::new(StateManager::new(tx));
            let mock_podman = Arc::new(MockPodmanExecutor::new());
            mock_podman.set_rm_result(Ok(()));
            mock_podman.set_rmi_result(Ok(()));

            // Create an adapter that was stopped "just now".
            let entry = crate::adapter::AdapterEntry {
                adapter_id: "timing-v1".to_string(),
                image_ref: "example.com/timing:v1".to_string(),
                checksum_sha256: "sha256:test".to_string(),
                state: AdapterState::Stopped,
                job_id: "job-timing".to_string(),
                stopped_at: Some(std::time::Instant::now()),
                error_message: None,
            };
            state_mgr.create_adapter(entry);

            // Run offload with the timeout — adapter should NOT be offloaded.
            crate::offload::run_offload_cycle(
                state_mgr.clone(),
                mock_podman.clone(),
                std::time::Duration::from_secs(timeout_secs),
            )
            .await;

            let adapter = state_mgr.get_adapter("timing-v1");
            prop_assert!(
                adapter.is_some(),
                "adapter should still exist before timeout"
            );
            prop_assert_eq!(
                adapter.unwrap().state,
                AdapterState::Stopped,
                "adapter should still be STOPPED"
            );
            Ok(())
        })?;
    }
}
