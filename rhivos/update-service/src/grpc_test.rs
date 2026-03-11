use std::sync::Arc;
use tokio::net::TcpListener;
use tonic::transport::{Channel, Server};

use crate::container::MockContainerRuntime;
use crate::manager::AdapterManager;
use crate::oci::{MockOciPuller, PullResult};

use super::proto::update_service_client::UpdateServiceClient;
use super::proto::*;
use super::UpdateServiceImpl;

/// Helper: compute SHA-256 checksum of a digest string for test fixtures.
fn compute_test_checksum(digest: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(digest.as_bytes());
    format!("sha256:{}", hex::encode(hash))
}

/// Helper: start an in-process gRPC server and return a connected client.
async fn start_test_server(
    oci: MockOciPuller,
    container: MockContainerRuntime,
) -> UpdateServiceClient<Channel> {
    let manager = Arc::new(AdapterManager::new(Arc::new(oci), Arc::new(container)));
    let service = UpdateServiceImpl::new(manager);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        Server::builder()
            .add_service(service.into_server())
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    // Give the server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let channel = Channel::from_shared(format!("http://127.0.0.1:{}", addr.port()))
        .unwrap()
        .connect()
        .await
        .unwrap();

    UpdateServiceClient::new(channel)
}

/// Helper: create mock OCI puller that returns a successful pull.
fn mock_oci_success(digest: &str) -> MockOciPuller {
    let digest = digest.to_string();
    let mut mock = MockOciPuller::new();
    mock.expect_pull_image()
        .returning(move |_| Ok(PullResult { digest: digest.clone() }));
    mock.expect_remove_image().returning(|_| Ok(()));
    mock
}

/// Helper: create mock container runtime that succeeds.
fn mock_container_success() -> MockContainerRuntime {
    let mut mock = MockContainerRuntime::new();
    mock.expect_run().returning(|_, _| Ok(()));
    mock.expect_stop().returning(|_| Ok(()));
    mock.expect_remove().returning(|_| Ok(()));
    mock.expect_status()
        .returning(|_| Ok(crate::container::ContainerStatus::Running));
    mock
}

// TS-07-1 + TS-07-2: gRPC install adapter and watch state transitions
#[tokio::test]
async fn test_grpc_install_and_watch() {
    let digest = "sha256:abc123";
    let checksum = compute_test_checksum(digest);

    let oci = mock_oci_success(digest);
    let container = mock_container_success();
    let mut client = start_test_server(oci, container).await;

    // Start watching for state events
    let mut watch_stream = client
        .watch_adapter_states(WatchAdapterStatesRequest {})
        .await
        .expect("watch_adapter_states should succeed")
        .into_inner();

    // Install the adapter
    let response = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "test-registry/image:v1.0".into(),
            checksum_sha256: checksum,
        })
        .await
        .expect("install_adapter should succeed")
        .into_inner();

    assert!(!response.job_id.is_empty(), "job_id should not be empty");
    assert!(!response.adapter_id.is_empty(), "adapter_id should not be empty");
    assert_eq!(
        response.state,
        AdapterState::Downloading as i32,
        "initial state should be DOWNLOADING"
    );

    // Collect state events — expect UNKNOWN->DOWNLOADING, DOWNLOADING->INSTALLING, INSTALLING->RUNNING
    let mut events = Vec::new();
    for _ in 0..3 {
        let event = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            watch_stream.message(),
        )
        .await
        .expect("should receive event within timeout")
        .expect("stream should not error")
        .expect("event should not be None");

        events.push(event);
    }

    assert_eq!(events.len(), 3, "should receive 3 state events");
    assert_eq!(events[0].old_state, AdapterState::Unknown as i32);
    assert_eq!(events[0].new_state, AdapterState::Downloading as i32);
    assert_eq!(events[1].old_state, AdapterState::Downloading as i32);
    assert_eq!(events[1].new_state, AdapterState::Installing as i32);
    assert_eq!(events[2].old_state, AdapterState::Installing as i32);
    assert_eq!(events[2].new_state, AdapterState::Running as i32);

    for event in &events {
        assert!(!event.adapter_id.is_empty(), "event adapter_id should not be empty");
        assert!(event.timestamp > 0, "event timestamp should be positive");
    }
}

// TS-07-5: gRPC list adapters
#[tokio::test]
async fn test_grpc_list_adapters() {
    let digest = "sha256:abc123";
    let checksum = compute_test_checksum(digest);

    let oci = mock_oci_success(digest);
    let container = mock_container_success();
    let mut client = start_test_server(oci, container).await;

    // Install an adapter first
    client
        .install_adapter(InstallAdapterRequest {
            image_ref: "test-registry/image:v1.0".into(),
            checksum_sha256: checksum,
        })
        .await
        .expect("install_adapter should succeed");

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // List adapters
    let response = client
        .list_adapters(ListAdaptersRequest {})
        .await
        .expect("list_adapters should succeed")
        .into_inner();

    assert!(!response.adapters.is_empty(), "should have at least one adapter");
    let adapter = &response.adapters[0];
    assert!(!adapter.adapter_id.is_empty());
    assert_eq!(adapter.image_ref, "test-registry/image:v1.0");
    assert_eq!(adapter.state, AdapterState::Running as i32);
}

// TS-07-6: gRPC get adapter status
#[tokio::test]
async fn test_grpc_get_status() {
    let digest = "sha256:abc123";
    let checksum = compute_test_checksum(digest);

    let oci = mock_oci_success(digest);
    let container = mock_container_success();
    let mut client = start_test_server(oci, container).await;

    let install_response = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "test-registry/image:v1.0".into(),
            checksum_sha256: checksum,
        })
        .await
        .expect("install_adapter should succeed")
        .into_inner();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let status = client
        .get_adapter_status(GetAdapterStatusRequest {
            adapter_id: install_response.adapter_id.clone(),
        })
        .await
        .expect("get_adapter_status should succeed")
        .into_inner();

    assert_eq!(status.adapter_id, install_response.adapter_id);
    assert_eq!(status.image_ref, "test-registry/image:v1.0");
    assert_eq!(status.state, AdapterState::Running as i32);
}

// TS-07-7: gRPC remove adapter
#[tokio::test]
async fn test_grpc_remove_adapter() {
    let digest = "sha256:abc123";
    let checksum = compute_test_checksum(digest);

    let oci = mock_oci_success(digest);
    let container = mock_container_success();
    let mut client = start_test_server(oci, container).await;

    let install_response = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "test-registry/image:v1.0".into(),
            checksum_sha256: checksum,
        })
        .await
        .expect("install_adapter should succeed")
        .into_inner();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let remove_response = client
        .remove_adapter(RemoveAdapterRequest {
            adapter_id: install_response.adapter_id.clone(),
        })
        .await
        .expect("remove_adapter should succeed")
        .into_inner();

    assert!(remove_response.success, "remove should succeed");

    // Adapter should no longer be listed
    let list_response = client
        .list_adapters(ListAdaptersRequest {})
        .await
        .expect("list_adapters should succeed")
        .into_inner();

    let found = list_response
        .adapters
        .iter()
        .any(|a| a.adapter_id == install_response.adapter_id);
    assert!(!found, "removed adapter should not be in the list");
}

// TS-07-E4: gRPC get status for unknown adapter
#[tokio::test]
async fn test_grpc_get_status_not_found() {
    let oci = MockOciPuller::new();
    let container = MockContainerRuntime::new();
    let mut client = start_test_server(oci, container).await;

    let result = client
        .get_adapter_status(GetAdapterStatusRequest {
            adapter_id: "nonexistent-adapter".into(),
        })
        .await;

    assert!(result.is_err(), "should return error for unknown adapter");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound, "should be NOT_FOUND");
}

// TS-07-E5: gRPC remove unknown adapter
#[tokio::test]
async fn test_grpc_remove_not_found() {
    let oci = MockOciPuller::new();
    let container = MockContainerRuntime::new();
    let mut client = start_test_server(oci, container).await;

    let result = client
        .remove_adapter(RemoveAdapterRequest {
            adapter_id: "nonexistent-adapter".into(),
        })
        .await;

    assert!(result.is_err(), "should return error for unknown adapter");
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound, "should be NOT_FOUND");
}
