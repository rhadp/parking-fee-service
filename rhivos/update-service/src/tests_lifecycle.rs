//! Lifecycle tests: single-adapter constraint (TS-07-7, TS-07-E6),
//! offload timer (TS-07-13, TS-07-E12), container monitor (TS-07-15,
//! TS-07-16, TS-07-E16), and removal failure (TS-07-E11).

use std::sync::Arc;

use tokio::sync::broadcast;
use tonic::Code;

use crate::adapter::{AdapterState, AdapterStateEvent};
use crate::grpc::proto;
use crate::grpc::proto::update_service_server::UpdateService;
use crate::grpc::UpdateServiceImpl;
use crate::monitor::monitor_container;
use crate::offload::offload_adapter;
use crate::podman::mock::MockPodmanExecutor;
use crate::podman::PodmanError;
use crate::state::StateManager;

/// Helper: create a test service with a mock podman executor.
fn test_service(
    mock: Arc<MockPodmanExecutor>,
) -> (
    UpdateServiceImpl,
    Arc<StateManager>,
    broadcast::Sender<AdapterStateEvent>,
) {
    let (tx, _rx) = broadcast::channel(64);
    let state_mgr = Arc::new(StateManager::new(tx.clone()));
    let svc = UpdateServiceImpl::new(state_mgr.clone(), mock, tx.clone());
    (svc, state_mgr, tx)
}

/// Helper: install an adapter and wait for it to reach RUNNING state.
async fn install_and_wait(
    svc: &UpdateServiceImpl,
    image_ref: &str,
    checksum: &str,
) {
    let request = tonic::Request::new(proto::InstallAdapterRequest {
        image_ref: image_ref.to_string(),
        checksum_sha256: checksum.to_string(),
    });
    svc.install_adapter(request).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
}

// TS-07-7: Single adapter constraint - stops running adapter before starting new one
#[tokio::test]
async fn test_single_adapter_stops_running() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Ok(()));
    mock.set_inspect_result(Ok("sha256:aaa".to_string()));
    mock.set_run_result(Ok(()));
    mock.set_stop_result(Ok(()));
    let (svc, state_mgr, _tx) = test_service(mock.clone());

    // Install adapter A
    install_and_wait(
        &svc,
        "example.com/adapter-a:v1",
        "sha256:aaa",
    )
    .await;

    let adapter_a = state_mgr.get_adapter("adapter-a-v1");
    assert!(adapter_a.is_some(), "adapter A should exist");

    // Now install adapter B (should stop A first)
    mock.set_inspect_result(Ok("sha256:bbb".to_string()));
    install_and_wait(
        &svc,
        "example.com/adapter-b:v2",
        "sha256:bbb",
    )
    .await;

    // Adapter A should have been stopped
    let stop_calls = mock.stop_calls();
    assert!(
        stop_calls.contains(&"adapter-a-v1".to_string()),
        "should have called stop on adapter A"
    );

    let adapter_a = state_mgr.get_adapter("adapter-a-v1").unwrap();
    assert_eq!(
        adapter_a.state,
        AdapterState::Stopped,
        "adapter A should be STOPPED"
    );

    let adapter_b = state_mgr.get_adapter("adapter-b-v2").unwrap();
    assert_eq!(
        adapter_b.state,
        AdapterState::Running,
        "adapter B should be RUNNING, got {:?}",
        adapter_b.state
    );
}

// TS-07-E6: Stop running adapter fails but install proceeds
#[tokio::test]
async fn test_stop_failure_install_proceeds() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Ok(()));
    mock.set_inspect_result(Ok("sha256:aaa".to_string()));
    mock.set_run_result(Ok(()));
    let (svc, state_mgr, _tx) = test_service(mock.clone());

    // Install adapter A
    install_and_wait(
        &svc,
        "example.com/adapter-a:v1",
        "sha256:aaa",
    )
    .await;

    // Configure stop to fail for adapter A
    mock.set_stop_result(Err(PodmanError::new("timeout")));
    mock.set_inspect_result(Ok("sha256:bbb".to_string()));

    // Install adapter B (stop A should fail but install proceeds)
    install_and_wait(
        &svc,
        "example.com/adapter-b:v2",
        "sha256:bbb",
    )
    .await;

    let adapter_a = state_mgr.get_adapter("adapter-a-v1").unwrap();
    assert_eq!(
        adapter_a.state,
        AdapterState::Error,
        "adapter A should be ERROR after failed stop"
    );

    let adapter_b = state_mgr.get_adapter("adapter-b-v2").unwrap();
    assert_eq!(
        adapter_b.state,
        AdapterState::Running,
        "adapter B should be RUNNING, got {:?}",
        adapter_b.state
    );
}

// TS-07-13: Offload timer triggers after inactivity timeout
// Verifies: STOPPED→OFFLOADING transition, rm/rmi calls, state removal,
// and event emission to subscribers (07-REQ-6.1 through 07-REQ-6.4).
#[tokio::test]
async fn test_offload_after_timeout() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_rm_result(Ok(()));
    mock.set_rmi_result(Ok(()));

    let (tx, _rx) = broadcast::channel(64);
    let state_mgr = Arc::new(StateManager::new(tx.clone()));

    // Create an adapter in STOPPED state
    let entry = crate::adapter::AdapterEntry {
        adapter_id: "adapter-v1".to_string(),
        image_ref: "example.com/adapter:v1".to_string(),
        checksum_sha256: "sha256:test".to_string(),
        state: AdapterState::Unknown,
        job_id: "test-job".to_string(),
        stopped_at: None,
        error_message: None,
    };
    state_mgr.create_adapter(entry);
    state_mgr
        .transition("adapter-v1", AdapterState::Downloading, None)
        .unwrap();
    state_mgr
        .transition("adapter-v1", AdapterState::Installing, None)
        .unwrap();
    state_mgr
        .transition("adapter-v1", AdapterState::Running, None)
        .unwrap();
    state_mgr
        .transition("adapter-v1", AdapterState::Stopped, None)
        .unwrap();

    // Subscribe AFTER setup transitions so we only get offload events
    let mut event_rx = tx.subscribe();

    // Run offload on the adapter
    offload_adapter(&state_mgr, mock.as_ref(), "adapter-v1", "example.com/adapter:v1").await;

    // Adapter should be removed from state
    assert!(
        state_mgr.get_adapter("adapter-v1").is_none(),
        "adapter should be removed after offload"
    );
    assert!(
        mock.rm_calls().contains(&"adapter-v1".to_string()),
        "podman rm should have been called"
    );
    assert!(
        mock.rmi_calls().contains(&"example.com/adapter:v1".to_string()),
        "podman rmi should have been called"
    );

    // Verify STOPPED→OFFLOADING event was emitted (07-REQ-6.4)
    let event = event_rx.try_recv().expect("should have received OFFLOADING event");
    assert_eq!(event.adapter_id, "adapter-v1");
    assert_eq!(event.old_state, AdapterState::Stopped);
    assert_eq!(event.new_state, AdapterState::Offloading);
    assert!(event.timestamp > 0);
}

// TS-07-13 (integration): The offload timer background loop detects
// expired STOPPED adapters and offloads them automatically.
#[tokio::test]
async fn test_offload_timer_fires_for_expired_adapter() {
    use std::time::{Duration, Instant};

    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_rm_result(Ok(()));
    mock.set_rmi_result(Ok(()));

    let (tx, _rx) = broadcast::channel(64);
    let state_mgr = Arc::new(StateManager::new(tx.clone()));

    // Create adapter and transition to STOPPED
    let entry = crate::adapter::AdapterEntry {
        adapter_id: "timer-test-v1".to_string(),
        image_ref: "example.com/timer-test:v1".to_string(),
        checksum_sha256: "sha256:test".to_string(),
        state: AdapterState::Unknown,
        job_id: "test-job".to_string(),
        stopped_at: None,
        error_message: None,
    };
    state_mgr.create_adapter(entry);
    state_mgr
        .transition("timer-test-v1", AdapterState::Downloading, None)
        .unwrap();
    state_mgr
        .transition("timer-test-v1", AdapterState::Installing, None)
        .unwrap();
    state_mgr
        .transition("timer-test-v1", AdapterState::Running, None)
        .unwrap();
    state_mgr
        .transition("timer-test-v1", AdapterState::Stopped, None)
        .unwrap();

    // Backdate stopped_at so the adapter appears to have been stopped
    // longer than the inactivity timeout
    let inactivity_timeout = Duration::from_secs(2);
    state_mgr.set_stopped_at(
        "timer-test-v1",
        Instant::now() - inactivity_timeout - Duration::from_secs(1),
    );

    // Subscribe for offload events
    let mut event_rx = tx.subscribe();

    // Start the offload timer with a short check interval
    let podman: Arc<dyn crate::podman::PodmanExecutor> = mock.clone();
    let timer_handle = tokio::spawn(crate::offload::run_offload_timer(
        state_mgr.clone(),
        podman,
        inactivity_timeout,
        Duration::from_millis(100),
    ));

    // Wait for the timer to fire
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Cancel the infinite timer loop
    timer_handle.abort();

    // Adapter should have been offloaded
    assert!(
        state_mgr.get_adapter("timer-test-v1").is_none(),
        "adapter should be removed after offload timer fires"
    );
    assert!(
        mock.rm_calls().contains(&"timer-test-v1".to_string()),
        "podman rm should have been called by offload timer"
    );
    assert!(
        mock.rmi_calls().contains(&"example.com/timer-test:v1".to_string()),
        "podman rmi should have been called by offload timer"
    );

    // Verify STOPPED→OFFLOADING event was emitted
    let event = event_rx.try_recv().expect("should have received OFFLOADING event");
    assert_eq!(event.old_state, AdapterState::Stopped);
    assert_eq!(event.new_state, AdapterState::Offloading);
}

// TS-07-13 (boundary): Adapter NOT yet past the inactivity timeout is
// not offloaded by the timer.
#[tokio::test]
async fn test_offload_timer_does_not_fire_before_timeout() {
    use std::time::Duration;

    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_rm_result(Ok(()));
    mock.set_rmi_result(Ok(()));

    let (tx, _rx) = broadcast::channel(64);
    let state_mgr = Arc::new(StateManager::new(tx));

    // Create adapter and transition to STOPPED (stopped_at = now)
    let entry = crate::adapter::AdapterEntry {
        adapter_id: "fresh-stop-v1".to_string(),
        image_ref: "example.com/fresh-stop:v1".to_string(),
        checksum_sha256: "sha256:test".to_string(),
        state: AdapterState::Unknown,
        job_id: "test-job".to_string(),
        stopped_at: None,
        error_message: None,
    };
    state_mgr.create_adapter(entry);
    state_mgr
        .transition("fresh-stop-v1", AdapterState::Downloading, None)
        .unwrap();
    state_mgr
        .transition("fresh-stop-v1", AdapterState::Installing, None)
        .unwrap();
    state_mgr
        .transition("fresh-stop-v1", AdapterState::Running, None)
        .unwrap();
    state_mgr
        .transition("fresh-stop-v1", AdapterState::Stopped, None)
        .unwrap();

    // Start the offload timer with a long inactivity timeout
    let inactivity_timeout = Duration::from_secs(3600); // 1 hour
    let podman: Arc<dyn crate::podman::PodmanExecutor> = mock.clone();
    let timer_handle = tokio::spawn(crate::offload::run_offload_timer(
        state_mgr.clone(),
        podman,
        inactivity_timeout,
        Duration::from_millis(100),
    ));

    // Let the timer run a few cycles
    tokio::time::sleep(Duration::from_millis(400)).await;

    // Cancel the timer
    timer_handle.abort();

    // Adapter should still exist (not offloaded)
    let adapter = state_mgr.get_adapter("fresh-stop-v1");
    assert!(adapter.is_some(), "adapter should NOT be offloaded before timeout");
    assert_eq!(
        adapter.unwrap().state,
        AdapterState::Stopped,
        "adapter should still be STOPPED"
    );

    // No rm/rmi calls should have been made for this adapter
    assert!(
        !mock.rm_calls().contains(&"fresh-stop-v1".to_string()),
        "podman rm should NOT have been called before timeout"
    );
}

// TS-07-E12: Offload cleanup failure transitions to ERROR
#[tokio::test]
async fn test_offload_failure_error() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_rm_result(Ok(()));
    mock.set_rmi_result(Err(PodmanError::new("image in use")));

    let (tx, _rx) = broadcast::channel(64);
    let state_mgr = Arc::new(StateManager::new(tx));

    // Create adapter in STOPPED state
    let entry = crate::adapter::AdapterEntry {
        adapter_id: "adapter-v1".to_string(),
        image_ref: "example.com/adapter:v1".to_string(),
        checksum_sha256: "sha256:test".to_string(),
        state: AdapterState::Unknown,
        job_id: "test-job".to_string(),
        stopped_at: None,
        error_message: None,
    };
    state_mgr.create_adapter(entry);
    state_mgr
        .transition("adapter-v1", AdapterState::Downloading, None)
        .unwrap();
    state_mgr
        .transition("adapter-v1", AdapterState::Installing, None)
        .unwrap();
    state_mgr
        .transition("adapter-v1", AdapterState::Running, None)
        .unwrap();
    state_mgr
        .transition("adapter-v1", AdapterState::Stopped, None)
        .unwrap();

    offload_adapter(&state_mgr, mock.as_ref(), "adapter-v1", "example.com/adapter:v1").await;

    // Adapter should be in ERROR state, not removed
    let adapter = state_mgr.get_adapter("adapter-v1");
    assert!(adapter.is_some(), "adapter should still exist after failed offload");
    assert_eq!(
        adapter.unwrap().state,
        AdapterState::Error,
        "adapter should be in ERROR state after offload failure"
    );
}

// TS-07-15: Container exit non-zero transitions to ERROR
#[tokio::test]
async fn test_container_exit_nonzero_error() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_wait_result(Ok(1)); // non-zero exit

    let (tx, _rx) = broadcast::channel(64);
    let state_mgr = Arc::new(StateManager::new(tx));

    // Create adapter in RUNNING state
    let entry = crate::adapter::AdapterEntry {
        adapter_id: "adapter-v1".to_string(),
        image_ref: "example.com/adapter:v1".to_string(),
        checksum_sha256: "sha256:test".to_string(),
        state: AdapterState::Unknown,
        job_id: "test-job".to_string(),
        stopped_at: None,
        error_message: None,
    };
    state_mgr.create_adapter(entry);
    state_mgr
        .transition("adapter-v1", AdapterState::Downloading, None)
        .unwrap();
    state_mgr
        .transition("adapter-v1", AdapterState::Installing, None)
        .unwrap();
    state_mgr
        .transition("adapter-v1", AdapterState::Running, None)
        .unwrap();

    let podman: Arc<dyn crate::podman::PodmanExecutor> = mock;
    monitor_container(state_mgr.clone(), podman, "adapter-v1".to_string()).await;

    let adapter = state_mgr.get_adapter("adapter-v1").unwrap();
    assert_eq!(
        adapter.state,
        AdapterState::Error,
        "adapter should be ERROR after non-zero exit"
    );
}

// TS-07-16: Container exit code 0 transitions to STOPPED
#[tokio::test]
async fn test_container_exit_zero_stopped() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_wait_result(Ok(0)); // clean exit

    let (tx, _rx) = broadcast::channel(64);
    let state_mgr = Arc::new(StateManager::new(tx));

    // Create adapter in RUNNING state
    let entry = crate::adapter::AdapterEntry {
        adapter_id: "adapter-v1".to_string(),
        image_ref: "example.com/adapter:v1".to_string(),
        checksum_sha256: "sha256:test".to_string(),
        state: AdapterState::Unknown,
        job_id: "test-job".to_string(),
        stopped_at: None,
        error_message: None,
    };
    state_mgr.create_adapter(entry);
    state_mgr
        .transition("adapter-v1", AdapterState::Downloading, None)
        .unwrap();
    state_mgr
        .transition("adapter-v1", AdapterState::Installing, None)
        .unwrap();
    state_mgr
        .transition("adapter-v1", AdapterState::Running, None)
        .unwrap();

    let podman: Arc<dyn crate::podman::PodmanExecutor> = mock;
    monitor_container(state_mgr.clone(), podman, "adapter-v1".to_string()).await;

    let adapter = state_mgr.get_adapter("adapter-v1").unwrap();
    assert_eq!(
        adapter.state,
        AdapterState::Stopped,
        "adapter should be STOPPED after clean exit"
    );
}

// TS-07-E16: Podman wait failure transitions to ERROR
#[tokio::test]
async fn test_podman_wait_failure_error() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_wait_result(Err(PodmanError::new("connection lost")));

    let (tx, _rx) = broadcast::channel(64);
    let state_mgr = Arc::new(StateManager::new(tx));

    // Create adapter in RUNNING state
    let entry = crate::adapter::AdapterEntry {
        adapter_id: "adapter-v1".to_string(),
        image_ref: "example.com/adapter:v1".to_string(),
        checksum_sha256: "sha256:test".to_string(),
        state: AdapterState::Unknown,
        job_id: "test-job".to_string(),
        stopped_at: None,
        error_message: None,
    };
    state_mgr.create_adapter(entry);
    state_mgr
        .transition("adapter-v1", AdapterState::Downloading, None)
        .unwrap();
    state_mgr
        .transition("adapter-v1", AdapterState::Installing, None)
        .unwrap();
    state_mgr
        .transition("adapter-v1", AdapterState::Running, None)
        .unwrap();

    let podman: Arc<dyn crate::podman::PodmanExecutor> = mock;
    monitor_container(state_mgr.clone(), podman, "adapter-v1".to_string()).await;

    let adapter = state_mgr.get_adapter("adapter-v1").unwrap();
    assert_eq!(
        adapter.state,
        AdapterState::Error,
        "adapter should be ERROR after podman wait failure"
    );
}

// TS-07-E8 (gRPC level): GetAdapterStatus with unknown adapter_id returns NOT_FOUND.
#[tokio::test]
async fn test_get_unknown_adapter_grpc() {
    let mock = Arc::new(MockPodmanExecutor::new());
    let (svc, _state_mgr, _tx) = test_service(mock);

    let request = tonic::Request::new(proto::GetAdapterStatusRequest {
        adapter_id: "nonexistent-adapter".to_string(),
    });

    let result = svc.get_adapter_status(request).await;
    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), Code::NotFound);
    assert!(
        status.message().contains("adapter not found"),
        "error message should contain 'adapter not found', got: {}",
        status.message()
    );
}

// TS-07-E10 (gRPC level): RemoveAdapter with unknown adapter_id returns NOT_FOUND.
#[tokio::test]
async fn test_remove_unknown_adapter_grpc() {
    let mock = Arc::new(MockPodmanExecutor::new());
    let (svc, _state_mgr, _tx) = test_service(mock);

    let request = tonic::Request::new(proto::RemoveAdapterRequest {
        adapter_id: "nonexistent-adapter".to_string(),
    });

    let result = svc.remove_adapter(request).await;
    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), Code::NotFound);
    assert!(
        status.message().contains("adapter not found"),
        "error message should contain 'adapter not found', got: {}",
        status.message()
    );
}

// TS-07-12 (gRPC level): RemoveAdapter on a running adapter calls stop, rm, rmi
// and removes the adapter from state.
#[tokio::test]
async fn test_remove_adapter_full_cleanup() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Ok(()));
    mock.set_inspect_result(Ok("sha256:abc".to_string()));
    mock.set_run_result(Ok(()));
    mock.set_stop_result(Ok(()));
    mock.set_rm_result(Ok(()));
    mock.set_rmi_result(Ok(()));

    let (svc, state_mgr, _tx) = test_service(mock.clone());

    let image_ref = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
    install_and_wait(&svc, image_ref, "sha256:abc").await;

    let adapter_id = "parkhaus-munich-v1.0.0";
    assert!(
        state_mgr.get_adapter(adapter_id).is_some(),
        "adapter should exist before removal"
    );

    let request = tonic::Request::new(proto::RemoveAdapterRequest {
        adapter_id: adapter_id.to_string(),
    });

    let result = svc.remove_adapter(request).await;
    assert!(result.is_ok(), "remove_adapter should succeed");

    // Verify podman calls
    assert!(
        mock.stop_calls().contains(&adapter_id.to_string()),
        "podman stop should have been called"
    );
    assert!(
        mock.rm_calls().contains(&adapter_id.to_string()),
        "podman rm should have been called"
    );
    assert!(
        mock.rmi_calls().contains(&image_ref.to_string()),
        "podman rmi should have been called"
    );

    // Adapter should be removed from state
    assert!(
        state_mgr.get_adapter(adapter_id).is_none(),
        "adapter should be removed from state after RemoveAdapter"
    );
}

// TS-07-8 (gRPC level): WatchAdapterStates streams events for state transitions.
#[tokio::test]
async fn test_watch_adapter_states_stream() {
    use tokio_stream::StreamExt;

    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Ok(()));
    mock.set_inspect_result(Ok("sha256:abc123".to_string()));
    mock.set_run_result(Ok(()));
    let (svc, _state_mgr, _tx) = test_service(mock);

    // Subscribe before install
    let watch_request = tonic::Request::new(proto::WatchAdapterStatesRequest {
        adapter_id: String::new(),
    });
    let resp = svc.watch_adapter_states(watch_request).await.unwrap();
    let mut stream = resp.into_inner();

    // Install an adapter to trigger state transitions
    let install_request = tonic::Request::new(proto::InstallAdapterRequest {
        image_ref: "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0".to_string(),
        checksum_sha256: "sha256:abc123".to_string(),
    });
    svc.install_adapter(install_request).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // Collect events from the stream (with timeout to avoid blocking)
    let mut events = Vec::new();
    loop {
        match tokio::time::timeout(std::time::Duration::from_millis(200), stream.next()).await {
            Ok(Some(Ok(event))) => events.push(event),
            _ => break,
        }
    }

    // Should have at least 3 events: UNKNOWN→DOWNLOADING, DOWNLOADING→INSTALLING, INSTALLING→RUNNING
    assert!(
        events.len() >= 3,
        "expected at least 3 state events, got {}",
        events.len()
    );

    // First event: UNKNOWN → DOWNLOADING
    assert_eq!(events[0].adapter_id, "parkhaus-munich-v1.0.0");
    assert_eq!(
        events[0].old_state(),
        proto::AdapterState::Unknown,
        "first event old_state should be Unknown"
    );
    assert_eq!(
        events[0].new_state(),
        proto::AdapterState::Downloading,
        "first event new_state should be Downloading"
    );
    assert!(events[0].timestamp > 0, "timestamp should be positive");

    // Second event: DOWNLOADING → INSTALLING
    assert_eq!(
        events[1].old_state(),
        proto::AdapterState::Downloading,
        "second event old_state should be Downloading"
    );
    assert_eq!(
        events[1].new_state(),
        proto::AdapterState::Installing,
        "second event new_state should be Installing"
    );

    // Third event: INSTALLING → RUNNING
    assert_eq!(
        events[2].old_state(),
        proto::AdapterState::Installing,
        "third event old_state should be Installing"
    );
    assert_eq!(
        events[2].new_state(),
        proto::AdapterState::Running,
        "third event new_state should be Running"
    );
}

// TS-07-10 (gRPC level): ListAdapters with one RUNNING and one STOPPED adapter.
#[tokio::test]
async fn test_list_adapters_with_states() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Ok(()));
    mock.set_inspect_result(Ok("sha256:aaa".to_string()));
    mock.set_run_result(Ok(()));
    mock.set_stop_result(Ok(()));
    let (svc, state_mgr, _tx) = test_service(mock.clone());

    // Install adapter A
    install_and_wait(&svc, "example.com/adapter-a:v1", "sha256:aaa").await;
    assert_eq!(
        state_mgr.get_adapter("adapter-a-v1").unwrap().state,
        AdapterState::Running,
        "adapter A should be RUNNING"
    );

    // Install adapter B (stops A → A is STOPPED, B is RUNNING)
    mock.set_inspect_result(Ok("sha256:bbb".to_string()));
    install_and_wait(&svc, "example.com/adapter-b:v2", "sha256:bbb").await;

    assert_eq!(
        state_mgr.get_adapter("adapter-a-v1").unwrap().state,
        AdapterState::Stopped,
        "adapter A should be STOPPED after B is installed"
    );
    assert_eq!(
        state_mgr.get_adapter("adapter-b-v2").unwrap().state,
        AdapterState::Running,
        "adapter B should be RUNNING"
    );

    // List adapters via gRPC
    let request = tonic::Request::new(proto::ListAdaptersRequest {});
    let resp = svc.list_adapters(request).await.unwrap();
    let adapters = &resp.into_inner().adapters;
    assert_eq!(adapters.len(), 2, "should have 2 adapters");

    let mut ids: Vec<String> = adapters.iter().map(|a| a.adapter_id.clone()).collect();
    ids.sort();
    assert_eq!(ids, vec!["adapter-a-v1", "adapter-b-v2"]);

    // Verify states in the response
    let adapter_a = adapters
        .iter()
        .find(|a| a.adapter_id == "adapter-a-v1")
        .unwrap();
    assert_eq!(
        adapter_a.state(),
        proto::AdapterState::Stopped,
        "adapter A should report STOPPED in ListAdapters"
    );

    let adapter_b = adapters
        .iter()
        .find(|a| a.adapter_id == "adapter-b-v2")
        .unwrap();
    assert_eq!(
        adapter_b.state(),
        proto::AdapterState::Running,
        "adapter B should report RUNNING in ListAdapters"
    );
}

// TS-07-11 (gRPC level): GetAdapterStatus returns the current state of a specific adapter.
#[tokio::test]
async fn test_get_adapter_status_grpc() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Ok(()));
    mock.set_inspect_result(Ok("sha256:abc123".to_string()));
    mock.set_run_result(Ok(()));
    let (svc, _state_mgr, _tx) = test_service(mock);

    install_and_wait(
        &svc,
        "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0",
        "sha256:abc123",
    )
    .await;

    let request = tonic::Request::new(proto::GetAdapterStatusRequest {
        adapter_id: "parkhaus-munich-v1.0.0".to_string(),
    });

    let resp = svc.get_adapter_status(request).await.unwrap();
    let adapter = resp.into_inner().adapter.unwrap();
    assert_eq!(adapter.adapter_id, "parkhaus-munich-v1.0.0");
    assert_eq!(
        adapter.state(),
        proto::AdapterState::Running,
        "adapter should report RUNNING via GetAdapterStatus"
    );
}

// TS-07-E11: Podman removal failure returns INTERNAL
#[tokio::test]
async fn test_removal_failure_internal() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Ok(()));
    mock.set_inspect_result(Ok("sha256:abc".to_string()));
    mock.set_run_result(Ok(()));
    mock.set_stop_result(Ok(()));
    mock.set_rm_result(Err(PodmanError::new("container in use")));

    let (svc, state_mgr, _tx) = test_service(mock);

    // Install an adapter first
    install_and_wait(&svc, "example.com/adapter:v1", "sha256:abc").await;

    // Try to remove it
    let request = tonic::Request::new(proto::RemoveAdapterRequest {
        adapter_id: "adapter-v1".to_string(),
    });

    let result = svc.remove_adapter(request).await;
    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), Code::Internal);

    let adapter = state_mgr.get_adapter("adapter-v1");
    assert!(adapter.is_some(), "adapter should still exist after failed removal");
    assert_eq!(
        adapter.unwrap().state,
        AdapterState::Error,
        "adapter should be in ERROR state after removal failure"
    );
}
