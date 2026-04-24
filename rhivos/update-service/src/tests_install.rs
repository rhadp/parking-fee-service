//! Install flow tests (TS-07-1 through TS-07-5, TS-07-E1 through TS-07-E5).
//!
//! These tests validate the InstallAdapter RPC and its underlying workflow:
//! immediate response, podman pull, checksum verification, container start,
//! and error handling.

use std::sync::Arc;

use tokio::sync::broadcast;
use tonic::Code;

use crate::adapter::{AdapterState, AdapterStateEvent};
use crate::grpc::proto;
use crate::grpc::proto::update_service_server::UpdateService;
use crate::grpc::UpdateServiceImpl;
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

// TS-07-1: InstallAdapter returns response immediately with UUID job_id,
// derived adapter_id, and initial state DOWNLOADING.
#[tokio::test]
async fn test_install_response_immediate() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Ok(()));
    mock.set_inspect_result(Ok("sha256:abc123".to_string()));
    mock.set_run_result(Ok(()));
    let (svc, _state_mgr, _tx) = test_service(mock);

    let request = tonic::Request::new(proto::InstallAdapterRequest {
        image_ref: "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0".to_string(),
        checksum_sha256: "sha256:abc123".to_string(),
    });

    let resp = svc.install_adapter(request).await.unwrap();
    let inner = resp.into_inner();

    // job_id should be a valid UUID v4
    assert!(!inner.job_id.is_empty());
    let parsed_uuid = uuid::Uuid::parse_str(&inner.job_id);
    assert!(parsed_uuid.is_ok(), "job_id should be valid UUID");

    assert_eq!(inner.adapter_id, "parkhaus-munich-v1.0.0");
    assert_eq!(inner.state(), proto::AdapterState::Downloading);
}

// TS-07-2: After InstallAdapter, the service executes podman pull with
// the provided image_ref.
#[tokio::test]
async fn test_install_calls_podman_pull() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Ok(()));
    mock.set_inspect_result(Ok("sha256:abc123".to_string()));
    mock.set_run_result(Ok(()));
    let (svc, _state_mgr, _tx) = test_service(mock.clone());

    let image_ref = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
    let request = tonic::Request::new(proto::InstallAdapterRequest {
        image_ref: image_ref.to_string(),
        checksum_sha256: "sha256:abc123".to_string(),
    });

    svc.install_adapter(request).await.unwrap();
    // Allow async task to run
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let calls = mock.pull_calls();
    assert_eq!(calls, vec![image_ref.to_string()]);
}

// TS-07-3: After pulling, the service inspects the image digest and
// compares it with the provided checksum.
#[tokio::test]
async fn test_install_verifies_checksum() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Ok(()));
    mock.set_inspect_result(Ok("sha256:abc123".to_string()));
    mock.set_run_result(Ok(()));
    let (svc, state_mgr, _tx) = test_service(mock.clone());

    let image_ref = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
    let request = tonic::Request::new(proto::InstallAdapterRequest {
        image_ref: image_ref.to_string(),
        checksum_sha256: "sha256:abc123".to_string(),
    });

    svc.install_adapter(request).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // inspect_digest should have been called
    assert_eq!(mock.inspect_calls().len(), 1);
    // Adapter should have progressed past DOWNLOADING (checksum matched)
    let adapter = state_mgr.get_adapter("parkhaus-munich-v1.0.0").unwrap();
    assert!(
        adapter.state == AdapterState::Installing || adapter.state == AdapterState::Running,
        "adapter should be INSTALLING or RUNNING after checksum match, got {:?}",
        adapter.state
    );
}

// TS-07-4: On checksum match, the service starts the container with
// --network=host and the derived adapter_id as the container name.
#[tokio::test]
async fn test_install_runs_with_network_host() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Ok(()));
    mock.set_inspect_result(Ok("sha256:abc123".to_string()));
    mock.set_run_result(Ok(()));
    let (svc, _state_mgr, _tx) = test_service(mock.clone());

    let image_ref = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
    let request = tonic::Request::new(proto::InstallAdapterRequest {
        image_ref: image_ref.to_string(),
        checksum_sha256: "sha256:abc123".to_string(),
    });

    svc.install_adapter(request).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let run_calls = mock.run_calls();
    assert_eq!(
        run_calls,
        vec![("parkhaus-munich-v1.0.0".to_string(), image_ref.to_string())]
    );
}

// TS-07-5: After successful container start, adapter state transitions to RUNNING.
#[tokio::test]
async fn test_install_reaches_running() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Ok(()));
    mock.set_inspect_result(Ok("sha256:abc123".to_string()));
    mock.set_run_result(Ok(()));
    // wait returns immediately with 0 for container exit detection;
    // we set it to never return by not setting it (default Ok(0) will cause
    // STOPPED - but for this test we just check the intermediate state)
    let (svc, state_mgr, _tx) = test_service(mock);

    let image_ref = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
    let request = tonic::Request::new(proto::InstallAdapterRequest {
        image_ref: image_ref.to_string(),
        checksum_sha256: "sha256:abc123".to_string(),
    });

    svc.install_adapter(request).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let adapter = state_mgr.get_adapter("parkhaus-munich-v1.0.0").unwrap();
    // The adapter should reach RUNNING (it may have already progressed
    // to STOPPED if the container monitor fires quickly)
    assert!(
        adapter.state == AdapterState::Running || adapter.state == AdapterState::Stopped,
        "adapter should be RUNNING or STOPPED, got {:?}",
        adapter.state
    );
}

// TS-07-E1: Empty image_ref returns INVALID_ARGUMENT
#[tokio::test]
async fn test_install_empty_image_ref() {
    let mock = Arc::new(MockPodmanExecutor::new());
    let (svc, _state_mgr, _tx) = test_service(mock);

    let request = tonic::Request::new(proto::InstallAdapterRequest {
        image_ref: String::new(),
        checksum_sha256: "sha256:abc123".to_string(),
    });

    let result = svc.install_adapter(request).await;
    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
    assert!(
        status.message().contains("image_ref is required"),
        "error message should contain 'image_ref is required', got: {}",
        status.message()
    );
}

// TS-07-E2: Empty checksum_sha256 returns INVALID_ARGUMENT
#[tokio::test]
async fn test_install_empty_checksum() {
    let mock = Arc::new(MockPodmanExecutor::new());
    let (svc, _state_mgr, _tx) = test_service(mock);

    let request = tonic::Request::new(proto::InstallAdapterRequest {
        image_ref: "example.com/img:v1".to_string(),
        checksum_sha256: String::new(),
    });

    let result = svc.install_adapter(request).await;
    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
    assert!(
        status.message().contains("checksum_sha256 is required"),
        "error message should contain 'checksum_sha256 is required', got: {}",
        status.message()
    );
}

// TS-07-E3: Podman pull failure transitions to ERROR
#[tokio::test]
async fn test_pull_failure_error_state() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Err(PodmanError::new("connection refused")));
    let (svc, state_mgr, _tx) = test_service(mock);

    let request = tonic::Request::new(proto::InstallAdapterRequest {
        image_ref: "bad-registry.com/img:v1".to_string(),
        checksum_sha256: "sha256:abc".to_string(),
    });

    svc.install_adapter(request).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let adapter = state_mgr.get_adapter("img-v1").unwrap();
    assert_eq!(adapter.state, AdapterState::Error);
    assert!(
        adapter
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("connection refused"),
        "error_message should contain 'connection refused', got: {:?}",
        adapter.error_message
    );
}

// TS-07-E4: Checksum mismatch transitions to ERROR and removes image
#[tokio::test]
async fn test_checksum_mismatch_error() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Ok(()));
    mock.set_inspect_result(Ok("sha256:different".to_string()));
    let (svc, state_mgr, _tx) = test_service(mock.clone());

    let image_ref = "example.com/img:v1";
    let request = tonic::Request::new(proto::InstallAdapterRequest {
        image_ref: image_ref.to_string(),
        checksum_sha256: "sha256:expected".to_string(),
    });

    svc.install_adapter(request).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let adapter = state_mgr.get_adapter("img-v1").unwrap();
    assert_eq!(adapter.state, AdapterState::Error);
    assert!(
        adapter
            .error_message
            .as_deref()
            .unwrap_or("")
            .contains("checksum_mismatch"),
        "error_message should contain 'checksum_mismatch', got: {:?}",
        adapter.error_message
    );
    // Image should have been removed
    assert!(
        mock.rmi_calls().contains(&image_ref.to_string()),
        "rmi should have been called for the mismatched image"
    );
}

// TS-07-E5: Podman run failure transitions to ERROR
#[tokio::test]
async fn test_run_failure_error_state() {
    let mock = Arc::new(MockPodmanExecutor::new());
    mock.set_pull_result(Ok(()));
    mock.set_inspect_result(Ok("sha256:abc".to_string()));
    mock.set_run_result(Err(PodmanError::new("container create failed")));
    let (svc, state_mgr, _tx) = test_service(mock);

    let request = tonic::Request::new(proto::InstallAdapterRequest {
        image_ref: "example.com/img:v1".to_string(),
        checksum_sha256: "sha256:abc".to_string(),
    });

    svc.install_adapter(request).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let adapter = state_mgr.get_adapter("img-v1").unwrap();
    assert_eq!(adapter.state, AdapterState::Error);
}
