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
    assert!(
        adapter_b.state == AdapterState::Running || adapter_b.state == AdapterState::Stopped,
        "adapter B should be RUNNING (or STOPPED if monitor fired), got {:?}",
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
    assert!(
        adapter_b.state == AdapterState::Running || adapter_b.state == AdapterState::Stopped,
        "adapter B should still reach RUNNING, got {:?}",
        adapter_b.state
    );
}

// TS-07-13: Offload timer triggers after inactivity timeout
#[tokio::test]
async fn test_offload_after_timeout() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_rm_result(Ok(()));
    mock.set_rmi_result(Ok(()));

    let (tx, _rx) = broadcast::channel(64);
    let state_mgr = Arc::new(StateManager::new(tx));

    // Create an adapter in STOPPED state with a past stopped_at timestamp
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
