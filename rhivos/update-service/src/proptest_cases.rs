//! Property-based tests for the update-service.

use crate::podman::PodmanExecutor;
use proptest::prelude::*;

// TS-07-P1: Adapter ID Determinism
proptest! {
    #[test]
    #[ignore]
    fn proptest_adapter_id_determinism(
        registry in "[a-z][a-z0-9.-]{0,20}",
        path in "[a-z][a-z0-9-]{0,15}",
        name in "[a-z][a-z0-9-]{1,15}",
        tag in "[a-z0-9][a-z0-9.-]{0,10}",
    ) {
        let image_ref = format!("{registry}/{path}/{name}:{tag}");
        let id1 = crate::adapter::derive_adapter_id(&image_ref);
        let id2 = crate::adapter::derive_adapter_id(&image_ref);
        prop_assert_eq!(&id1, &id2, "same input must produce same output");

        // Expected format: {name}-{tag}
        let expected = format!("{name}-{tag}");
        prop_assert_eq!(id1, expected);
    }
}

// TS-07-P2: Single Adapter Invariant
proptest! {
    #[test]
    #[ignore]
    fn proptest_single_adapter_invariant(
        count in 1usize..=4,
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            use crate::adapter::AdapterStateEvent;
            use crate::state::StateManager;
            use crate::testing::MockPodmanExecutor;
            use crate::service::UpdateServiceHandler;
            use crate::config::Config;
            use crate::adapter::AdapterState;
            use std::sync::Arc;
            use tokio::sync::broadcast;

            let (tx, _) = broadcast::channel::<AdapterStateEvent>(64);
            let state_mgr = Arc::new(StateManager::new(tx));
            let mock = MockPodmanExecutor::new();
            mock.set_pull_result(Ok(()));
            mock.set_run_result(Ok(()));

            let config = Config {
                grpc_port: 50052,
                registry_url: String::new(),
                inactivity_timeout_secs: 86400,
                container_storage_path: "/tmp/test/".to_string(),
            };
            let svc = UpdateServiceHandler::new(
                state_mgr.clone(),
                Arc::new(mock.clone()),
                config,
            );

            for i in 0..count {
                let image_ref = format!("example.com/adapter-{i}:v1");
                let checksum = format!("sha256:check{i}");
                mock.set_inspect_result(Ok(checksum.clone()));

                let _ = svc.install_adapter(&image_ref, &checksum).await;
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;

                let running_count = state_mgr
                    .list_adapters()
                    .iter()
                    .filter(|a| a.state == AdapterState::Running)
                    .count();
                prop_assert!(running_count <= 1, "at most one adapter should be RUNNING, got {running_count}");
            }
            Ok(())
        })?;
    }
}

// TS-07-P3: State Transition Validity
proptest! {
    #[test]
    #[ignore]
    fn proptest_state_transition_validity(
        _dummy in 0u8..5,
    ) {
        use crate::adapter::{AdapterState, AdapterStateEvent};

        let valid_transitions: Vec<(AdapterState, AdapterState)> = vec![
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

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            use crate::state::StateManager;
            use crate::adapter::AdapterEntry;
            use tokio::sync::broadcast;

            let (tx, _) = broadcast::channel::<AdapterStateEvent>(64);
            let state_mgr = StateManager::new(tx);
            let mut rx = state_mgr.subscribe();

            let entry = AdapterEntry {
                adapter_id: "test-adapter-v1".to_string(),
                image_ref: "example.com/test-adapter:v1".to_string(),
                checksum_sha256: "sha256:test".to_string(),
                state: AdapterState::Unknown,
                job_id: "job-1".to_string(),
                stopped_at: None,
                error_message: None,
            };
            state_mgr.create_adapter(entry);

            // Walk through the happy path
            for new_state in [
                AdapterState::Downloading,
                AdapterState::Installing,
                AdapterState::Running,
            ] {
                state_mgr.transition("test-adapter-v1", new_state, None).unwrap();
            }

            // Collect and validate all events
            while let Ok(event) = rx.try_recv() {
                let pair = (event.old_state.clone(), event.new_state.clone());
                prop_assert!(
                    valid_transitions.contains(&pair),
                    "invalid transition: {pair:?}"
                );
            }

            Ok(())
        })?;
    }
}

// TS-07-P4: Event Delivery Completeness
proptest! {
    #[test]
    #[ignore]
    fn proptest_event_delivery_completeness(
        n_subscribers in 1usize..=3,
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};
            use crate::state::StateManager;
            use tokio::sync::broadcast;

            let (tx, _) = broadcast::channel::<AdapterStateEvent>(64);
            let state_mgr = StateManager::new(tx);

            // Create N subscribers
            let mut receivers: Vec<_> = (0..n_subscribers)
                .map(|_| state_mgr.subscribe())
                .collect();

            // Create adapter and transition
            let entry = AdapterEntry {
                adapter_id: "test-adapter-v1".to_string(),
                image_ref: "example.com/test-adapter:v1".to_string(),
                checksum_sha256: "sha256:test".to_string(),
                state: AdapterState::Unknown,
                job_id: "job-1".to_string(),
                stopped_at: None,
                error_message: None,
            };
            state_mgr.create_adapter(entry);
            state_mgr.transition("test-adapter-v1", AdapterState::Downloading, None).unwrap();

            // All subscribers should get the same event
            let mut all_events = Vec::new();
            for rx in &mut receivers {
                let event = rx.try_recv().expect("each subscriber should get the event");
                all_events.push(event);
            }

            // All events should be identical
            for event in &all_events {
                prop_assert_eq!(event, &all_events[0], "all subscribers should get identical events");
            }

            Ok(())
        })?;
    }
}

// TS-07-P5: Checksum Verification Soundness
proptest! {
    #[test]
    #[ignore]
    fn proptest_checksum_verification_soundness(
        digest_suffix in "[a-f0-9]{8,16}",
        checksum_suffix in "[a-f0-9]{8,16}",
    ) {
        let digest = format!("sha256:{digest_suffix}");
        let checksum = format!("sha256:{checksum_suffix}");

        // Only test when they differ
        prop_assume!(digest != checksum);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            use crate::adapter::{AdapterState, AdapterStateEvent};
            use crate::config::Config;
            use crate::service::UpdateServiceHandler;
            use crate::state::StateManager;
            use crate::testing::MockPodmanExecutor;
            use std::sync::Arc;
            use tokio::sync::broadcast;

            let (tx, _) = broadcast::channel::<AdapterStateEvent>(64);
            let state_mgr = Arc::new(StateManager::new(tx));
            let mock = MockPodmanExecutor::new();
            mock.set_pull_result(Ok(()));
            mock.set_inspect_result(Ok(digest.clone()));
            mock.set_rmi_result(Ok(()));

            let config = Config {
                grpc_port: 50052,
                registry_url: String::new(),
                inactivity_timeout_secs: 86400,
                container_storage_path: "/tmp/test/".to_string(),
            };
            let svc = UpdateServiceHandler::new(
                state_mgr.clone(),
                Arc::new(mock.clone()),
                config,
            );

            let image_ref = "example.com/test-img:v1";
            let _ = svc.install_adapter(image_ref, &checksum).await;
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            let adapter = state_mgr.get_adapter("test-img-v1")
                .expect("adapter should exist");
            prop_assert_eq!(adapter.state, AdapterState::Error, "mismatched checksum should produce ERROR");
            prop_assert!(
                mock.rmi_calls().contains(&image_ref.to_string()),
                "image should be removed on mismatch"
            );

            Ok(())
        })?;
    }
}

// TS-07-P6: Offload Timing Correctness
proptest! {
    #[test]
    #[ignore]
    fn proptest_offload_timing_correctness(
        timeout_secs in 2u64..=5,
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};
            use crate::state::StateManager;
            use crate::testing::MockPodmanExecutor;
            use crate::offload::run_offload_check;
            use std::sync::Arc;
            use std::time::{Duration, Instant};
            use tokio::sync::broadcast;

            let (tx, _) = broadcast::channel::<AdapterStateEvent>(16);
            let state_mgr = Arc::new(StateManager::new(tx));
            let mock = MockPodmanExecutor::new();
            mock.set_rm_result(Ok(()));
            mock.set_rmi_result(Ok(()));

            // Create adapter STOPPED only 1 second ago
            let entry = AdapterEntry {
                adapter_id: "test-adapter-v1".to_string(),
                image_ref: "example.com/test-adapter:v1".to_string(),
                checksum_sha256: "sha256:test".to_string(),
                state: AdapterState::Stopped,
                job_id: "job-1".to_string(),
                stopped_at: Some(Instant::now() - Duration::from_secs(1)),
                error_message: None,
            };
            state_mgr.create_adapter(entry);

            // Run offload check with timeout_secs (which is >= 2)
            // Since the adapter has only been stopped for 1 second, it should NOT be offloaded
            let inactivity = Duration::from_secs(timeout_secs);
            run_offload_check(
                &state_mgr,
                &(Arc::new(mock) as Arc<dyn PodmanExecutor>),
                inactivity,
            ).await;

            let adapter = state_mgr.get_adapter("test-adapter-v1");
            prop_assert!(adapter.is_some(), "adapter should NOT be offloaded yet");
            prop_assert_eq!(adapter.unwrap().state, AdapterState::Stopped);

            Ok(())
        })?;
    }
}
