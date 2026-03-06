use std::sync::Arc;

use tokio_stream::StreamExt;
use tonic::transport::{Channel, Server};

use crate::container::MockContainerRuntime;
use crate::grpc::UpdateServiceImpl;
use crate::manager::AdapterManager;
use crate::oci::{MockOciPuller, PullResult};
use crate::proto::update_service_client::UpdateServiceClient;
use crate::proto::update_service_server::UpdateServiceServer;
use crate::proto;

/// Helper to create mocks and start an in-process gRPC server, returning a client.
async fn start_test_server() -> UpdateServiceClient<Channel> {
    let mut oci = MockOciPuller::new();
    oci.expect_pull_image().returning(|_| {
        Ok(PullResult {
            digest: "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                .to_string(),
        })
    });
    oci.expect_remove_image().returning(|_| Ok(()));

    let mut container = MockContainerRuntime::new();
    container.expect_run().returning(|_, _| Ok(()));
    container.expect_stop().returning(|_| Ok(()));
    container.expect_remove().returning(|_| Ok(()));
    container
        .expect_status()
        .returning(|_| Ok(crate::container::ContainerStatus::Running));

    let manager = Arc::new(AdapterManager::new(Arc::new(oci), Arc::new(container)));
    let service = UpdateServiceImpl::new(manager);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        Server::builder()
            .add_service(UpdateServiceServer::new(service))
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .unwrap();
    });

    // Give the server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    UpdateServiceClient::connect(format!("http://{addr}"))
        .await
        .unwrap()
}

/// Compute the valid checksum for the test digest.
fn valid_checksum() -> String {
    use sha2::{Digest, Sha256};
    let digest_str =
        "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
    let hash = Sha256::digest(digest_str.as_bytes());
    format!("sha256:{}", hex::encode(hash))
}

/// TS-07-1, TS-07-2: Install adapter and watch state transitions via gRPC.
#[tokio::test]
async fn test_grpc_install_and_watch() {
    let mut client = start_test_server().await;

    // Start watching before installing
    let mut watch_stream = client
        .watch_adapter_states(proto::WatchAdapterStatesRequest {})
        .await
        .unwrap()
        .into_inner();

    // Install adapter
    let response = client
        .install_adapter(proto::InstallAdapterRequest {
            image_ref: "test-image:v1.0".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap()
        .into_inner();

    assert!(!response.job_id.is_empty());
    assert!(!response.adapter_id.is_empty());

    // Collect state transition events
    let mut events = Vec::new();
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(Ok(event)) = watch_stream.next().await {
            let new_state = event.new_state;
            events.push(event);
            // Stop after reaching RUNNING
            if new_state == proto::AdapterState::Running as i32 {
                break;
            }
        }
    });

    let _ = timeout.await;

    // Verify event sequence
    assert!(
        events.len() >= 3,
        "expected at least 3 state events, got {}",
        events.len()
    );
}

/// TS-07-5: List adapters via gRPC.
#[tokio::test]
async fn test_grpc_list_adapters() {
    let mut client = start_test_server().await;

    // Initially empty
    let response = client
        .list_adapters(proto::ListAdaptersRequest {})
        .await
        .unwrap()
        .into_inner();
    assert!(
        response.adapters.is_empty(),
        "adapter list should be empty initially"
    );
}

/// TS-07-6: Get adapter status via gRPC.
#[tokio::test]
async fn test_grpc_get_status() {
    let mut client = start_test_server().await;

    // Query unknown adapter
    let result = client
        .get_adapter_status(proto::GetAdapterStatusRequest {
            adapter_id: "nonexistent".to_string(),
        })
        .await;

    assert!(result.is_err(), "should return error for unknown adapter");
    assert_eq!(
        result.unwrap_err().code(),
        tonic::Code::NotFound,
        "should return NOT_FOUND"
    );
}

/// TS-07-7: Remove adapter via gRPC.
#[tokio::test]
async fn test_grpc_remove_adapter() {
    let mut client = start_test_server().await;

    // Remove unknown adapter
    let result = client
        .remove_adapter(proto::RemoveAdapterRequest {
            adapter_id: "nonexistent".to_string(),
        })
        .await;

    assert!(result.is_err(), "should return error for unknown adapter");
    assert_eq!(
        result.unwrap_err().code(),
        tonic::Code::NotFound,
        "should return NOT_FOUND"
    );
}
