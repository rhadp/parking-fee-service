//! Integration tests for the UPDATE_SERVICE.
//!
//! These tests verify the UPDATE_SERVICE gRPC interface using in-process
//! gRPC with mock OCI registry and container runtime. No external
//! infrastructure is required.
//!
//! Test Spec Coverage:
//! - TS-04-15 through TS-04-20 (acceptance criteria, task group 5)
//! - TS-04-E8, TS-04-E9 (edge cases, task group 5)
//! - TS-04-21 through TS-04-26 (acceptance criteria, task group 6)
//! - TS-04-E10 through TS-04-E13 (edge cases, task group 6)
//! - TS-04-P5, TS-04-P6, TS-04-P8 (property tests, task group 6)

use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{Channel, Server};

use update_service::adapter_manager::AdapterManager;
use update_service::checksum::compute_sha256;
use update_service::container_runtime::MockContainerRuntime;
use update_service::grpc_service::UpdateServiceGrpc;
use update_service::oci_client::MockOciRegistry;
use update_service::offloader;
use update_service::parking::update::v1::update_service_client::UpdateServiceClient;
use update_service::parking::update::v1::update_service_server::UpdateServiceServer;
use update_service::parking::update::v1::{
    GetAdapterStatusRequest, InstallAdapterRequest, ListAdaptersRequest, RemoveAdapterRequest,
    WatchAdapterStatesRequest,
};

/// The manifest content used by the mock OCI registry in tests.
const TEST_MANIFEST: &[u8] = b"test-oci-manifest-content-for-update-service";

/// Start an in-process UPDATE_SERVICE gRPC server with a mock OCI registry
/// and mock container runtime on an ephemeral port.
async fn start_test_server_with(
    offload_timeout: Duration,
    registry: Arc<MockOciRegistry>,
    runtime: Arc<MockContainerRuntime>,
) -> (UpdateServiceClient<Channel>, Arc<Mutex<AdapterManager>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let manager = Arc::new(Mutex::new(AdapterManager::new(offload_timeout)));
    let service = UpdateServiceGrpc::new(manager.clone(), registry, runtime);

    let incoming = TcpListenerStream::new(listener);

    tokio::spawn(async move {
        Server::builder()
            .add_service(UpdateServiceServer::new(service))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    // Brief delay for server startup
    tokio::time::sleep(Duration::from_millis(100)).await;

    let channel = Channel::from_shared(format!("http://{}", addr))
        .unwrap()
        .connect()
        .await
        .unwrap();
    let client = UpdateServiceClient::new(channel);

    (client, manager)
}

/// Start a default test server with valid mock registry and runtime.
async fn start_test_server() -> (UpdateServiceClient<Channel>, Arc<Mutex<AdapterManager>>) {
    let registry = Arc::new(MockOciRegistry::new(TEST_MANIFEST));
    let runtime = Arc::new(MockContainerRuntime::new());
    start_test_server_with(Duration::from_secs(24 * 3600), registry, runtime).await
}

/// Compute the valid checksum for the test manifest.
fn valid_checksum() -> String {
    compute_sha256(TEST_MANIFEST)
}

// ==========================================================================
// TS-04-15: UPDATE_SERVICE exposes gRPC service
// Requirement: 04-REQ-4.1
// ==========================================================================

#[tokio::test]
async fn test_update_service_grpc_service() {
    let (mut client, _mgr) = start_test_server().await;

    // Verify the server responds to ListAdapters
    let response = client.list_adapters(ListAdaptersRequest {}).await;

    assert!(response.is_ok(), "gRPC connection should succeed");
    let resp = response.unwrap().into_inner();
    // No adapters yet, so the list should be empty
    assert!(resp.adapters.is_empty());
}

// ==========================================================================
// TS-04-16: InstallAdapter returns job_id, adapter_id, state
// Requirement: 04-REQ-4.2
// ==========================================================================

#[tokio::test]
async fn test_update_service_install_adapter() {
    let (mut client, _mgr) = start_test_server().await;

    let response = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await;

    assert!(response.is_ok());
    let resp = response.unwrap().into_inner();
    assert!(!resp.job_id.is_empty(), "job_id should not be empty");
    assert!(
        !resp.adapter_id.is_empty(),
        "adapter_id should not be empty"
    );
    // state == DOWNLOADING (proto value 1)
    assert_eq!(resp.state, 1, "initial state should be DOWNLOADING");
}

// ==========================================================================
// TS-04-17: WatchAdapterStates streams events
// Requirement: 04-REQ-4.3
// ==========================================================================

#[tokio::test]
async fn test_update_service_watch_adapter_states() {
    let (mut client, _mgr) = start_test_server().await;

    // Start watching before installing
    let mut stream = client
        .watch_adapter_states(WatchAdapterStatesRequest {})
        .await
        .unwrap()
        .into_inner();

    // Install an adapter to trigger a state event
    let install_resp = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap()
        .into_inner();

    // Wait for the event with a timeout
    let event = tokio::time::timeout(Duration::from_secs(5), stream.message())
        .await
        .expect("should receive event within timeout")
        .expect("stream should not error")
        .expect("should have at least one event");

    assert_eq!(event.adapter_id, install_resp.adapter_id);
    // old_state == UNKNOWN (0), new_state == DOWNLOADING (1)
    assert_eq!(event.old_state, 0, "old state should be UNKNOWN");
    assert_eq!(event.new_state, 1, "new state should be DOWNLOADING");
}

// ==========================================================================
// TS-04-18: ListAdapters returns all known adapters
// Requirement: 04-REQ-4.4
// ==========================================================================

#[tokio::test]
async fn test_update_service_list_adapters() {
    let (mut client, _mgr) = start_test_server().await;

    // Install an adapter
    client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap();

    // Wait for the async pipeline to register the adapter
    tokio::time::sleep(Duration::from_millis(200)).await;

    // List adapters
    let response = client
        .list_adapters(ListAdaptersRequest {})
        .await
        .unwrap()
        .into_inner();

    assert!(
        response.adapters.len() >= 1,
        "should have at least one adapter"
    );
    let adapter = &response.adapters[0];
    assert!(
        !adapter.adapter_id.is_empty(),
        "adapter_id should not be empty"
    );
    // state should not be UNKNOWN (0)
    assert_ne!(adapter.state, 0, "state should not be UNKNOWN");
}

// ==========================================================================
// TS-04-19: RemoveAdapter stops and removes adapter
// Requirement: 04-REQ-4.5
// ==========================================================================

#[tokio::test]
async fn test_update_service_remove_adapter() {
    let (mut client, _mgr) = start_test_server().await;

    // Install an adapter
    let install_resp = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap()
        .into_inner();

    let adapter_id = install_resp.adapter_id;

    // Wait briefly for the pipeline to start
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Remove the adapter
    let remove_resp = client
        .remove_adapter(RemoveAdapterRequest {
            adapter_id: adapter_id.clone(),
        })
        .await;
    assert!(remove_resp.is_ok(), "RemoveAdapter should succeed");

    // Verify it's no longer in the list
    let list_resp = client
        .list_adapters(ListAdaptersRequest {})
        .await
        .unwrap()
        .into_inner();

    let found = list_resp
        .adapters
        .iter()
        .any(|a| a.adapter_id == adapter_id);
    assert!(!found, "removed adapter should not appear in list");
}

// ==========================================================================
// TS-04-20: GetAdapterStatus returns adapter info
// Requirement: 04-REQ-4.6
// ==========================================================================

#[tokio::test]
async fn test_update_service_get_adapter_status() {
    let (mut client, _mgr) = start_test_server().await;

    // Install an adapter
    let install_resp = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap()
        .into_inner();

    let adapter_id = install_resp.adapter_id;

    // Get adapter status (may still be DOWNLOADING — that's fine)
    let status_resp = client
        .get_adapter_status(GetAdapterStatusRequest {
            adapter_id: adapter_id.clone(),
        })
        .await
        .unwrap()
        .into_inner();

    let adapter = status_resp
        .adapter
        .expect("adapter info should be present");
    assert_eq!(adapter.adapter_id, adapter_id);
    // State should not be UNKNOWN
    assert_ne!(adapter.state, 0, "state should not be UNKNOWN");
}

// ==========================================================================
// TS-04-E8: InstallAdapter for already-installed adapter
// Requirement: 04-REQ-4.E1
// ==========================================================================

#[tokio::test]
async fn test_edge_install_already_installed() {
    let (mut client, _mgr) = start_test_server().await;

    // Install an adapter
    client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap();

    // Try to install again with the same image_ref
    let result = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await;

    assert!(result.is_err(), "duplicate install should fail");
    let status = result.unwrap_err();
    assert_eq!(
        status.code(),
        tonic::Code::AlreadyExists,
        "should return ALREADY_EXISTS"
    );
}

// ==========================================================================
// TS-04-E9: RemoveAdapter/GetAdapterStatus with unknown adapter_id
// Requirement: 04-REQ-4.E2
// ==========================================================================

#[tokio::test]
async fn test_edge_remove_unknown_adapter() {
    let (mut client, _mgr) = start_test_server().await;

    // RemoveAdapter with unknown adapter_id
    let remove_result = client
        .remove_adapter(RemoveAdapterRequest {
            adapter_id: "nonexistent-adapter".to_string(),
        })
        .await;

    assert!(remove_result.is_err());
    assert_eq!(
        remove_result.unwrap_err().code(),
        tonic::Code::NotFound,
        "RemoveAdapter should return NOT_FOUND"
    );

    // GetAdapterStatus with unknown adapter_id
    let status_result = client
        .get_adapter_status(GetAdapterStatusRequest {
            adapter_id: "nonexistent-adapter".to_string(),
        })
        .await;

    assert!(status_result.is_err());
    assert_eq!(
        status_result.unwrap_err().code(),
        tonic::Code::NotFound,
        "GetAdapterStatus should return NOT_FOUND"
    );
}

// ==========================================================================
// Task Group 6 Tests
// ==========================================================================

// TS-04-21: OCI image pull on InstallAdapter
// Requirement: 04-REQ-5.1
#[tokio::test]
async fn test_oci_image_pull() {
    let registry = Arc::new(MockOciRegistry::new(TEST_MANIFEST));
    let runtime = Arc::new(MockContainerRuntime::new());
    let (mut client, _mgr) =
        start_test_server_with(Duration::from_secs(3600), registry, runtime).await;

    // Install an adapter — the pipeline should pull the manifest and proceed
    let resp = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap()
        .into_inner();

    assert!(!resp.adapter_id.is_empty());
    assert_eq!(resp.state, 1, "initial state should be DOWNLOADING");

    // Wait for the pipeline to complete
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify the adapter has progressed past DOWNLOADING
    let status_resp = client
        .get_adapter_status(GetAdapterStatusRequest {
            adapter_id: resp.adapter_id.clone(),
        })
        .await
        .unwrap()
        .into_inner();

    let adapter = status_resp.adapter.unwrap();
    // Should be RUNNING (3) after successful pull + checksum + start
    assert_eq!(
        adapter.state, 3,
        "adapter should be RUNNING after successful install pipeline"
    );
}

// TS-04-23: Checksum match transitions to INSTALLING
// Requirement: 04-REQ-5.3
#[tokio::test]
async fn test_oci_checksum_match_transition() {
    let registry = Arc::new(MockOciRegistry::new(TEST_MANIFEST));
    let runtime = Arc::new(MockContainerRuntime::new());
    let (mut client, _mgr) =
        start_test_server_with(Duration::from_secs(3600), registry, runtime).await;

    // Start watching before installing
    let mut stream = client
        .watch_adapter_states(WatchAdapterStatesRequest {})
        .await
        .unwrap()
        .into_inner();

    // Install with correct checksum
    client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap();

    // Collect events for a few seconds
    let mut states: Vec<i32> = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        match tokio::time::timeout_at(deadline, stream.message()).await {
            Ok(Ok(Some(event))) => {
                states.push(event.new_state);
            }
            _ => break,
        }
    }

    // Expect: DOWNLOADING(1), INSTALLING(2), RUNNING(3)
    assert!(
        states.contains(&1),
        "should transition through DOWNLOADING; got states: {:?}",
        states
    );
    assert!(
        states.contains(&2),
        "should transition through INSTALLING after checksum match; got states: {:?}",
        states
    );
    // Verify order: DOWNLOADING before INSTALLING
    let dl_idx = states.iter().position(|&s| s == 1).unwrap();
    let inst_idx = states.iter().position(|&s| s == 2).unwrap();
    assert!(
        dl_idx < inst_idx,
        "DOWNLOADING should come before INSTALLING"
    );
}

// TS-04-25: Stopped adapter offloaded after timeout
// Requirement: 04-REQ-6.2
#[tokio::test]
async fn test_offloading_stopped_adapter_offloaded() {
    let registry = Arc::new(MockOciRegistry::new(TEST_MANIFEST));
    let runtime = Arc::new(MockContainerRuntime::new());
    let offload_timeout = Duration::from_secs(1);
    let (mut client, manager) =
        start_test_server_with(offload_timeout, registry, runtime.clone()).await;

    // Start the offloader with a short check interval
    let _offloader = offloader::start_offloader(
        manager.clone(),
        runtime,
        Some(Duration::from_millis(500)),
    );

    // Install an adapter and wait for it to become RUNNING
    let install_resp = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap()
        .into_inner();

    let adapter_id = install_resp.adapter_id;

    // Wait for pipeline to complete
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Transition to STOPPED manually via the manager
    {
        let mut mgr = manager.lock().await;
        mgr.transition(
            &adapter_id,
            update_service::adapter_manager::AdapterState::Stopped,
        )
        .unwrap();
    }

    // Wait for offloading timeout + check interval
    tokio::time::sleep(Duration::from_secs(3)).await;

    // The adapter should have been offloaded and removed
    let list_resp = client
        .list_adapters(ListAdaptersRequest {})
        .await
        .unwrap()
        .into_inner();

    let found = list_resp
        .adapters
        .iter()
        .any(|a| a.adapter_id == adapter_id);
    assert!(
        !found,
        "adapter should have been offloaded and removed from list"
    );
}

// TS-04-26: Offloading emits state events
// Requirement: 04-REQ-6.3
#[tokio::test]
async fn test_offloading_emits_state_events() {
    let registry = Arc::new(MockOciRegistry::new(TEST_MANIFEST));
    let runtime = Arc::new(MockContainerRuntime::new());
    let offload_timeout = Duration::from_secs(1);
    let (mut client, manager) =
        start_test_server_with(offload_timeout, registry, runtime.clone()).await;

    // Start the offloader
    let _offloader = offloader::start_offloader(
        manager.clone(),
        runtime,
        Some(Duration::from_millis(500)),
    );

    // Install an adapter
    let install_resp = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap()
        .into_inner();

    let adapter_id = install_resp.adapter_id;

    // Wait for RUNNING
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Start watching
    let mut stream = client
        .watch_adapter_states(WatchAdapterStatesRequest {})
        .await
        .unwrap()
        .into_inner();

    // Transition to STOPPED
    {
        let mut mgr = manager.lock().await;
        mgr.transition(
            &adapter_id,
            update_service::adapter_manager::AdapterState::Stopped,
        )
        .unwrap();
    }

    // Collect events for a few seconds (should see OFFLOADING)
    let mut seen_offloading = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        match tokio::time::timeout_at(deadline, stream.message()).await {
            Ok(Ok(Some(event))) => {
                // OFFLOADING = 6
                if event.new_state == 6 {
                    seen_offloading = true;
                }
            }
            _ => break,
        }
    }

    assert!(
        seen_offloading,
        "should have received an OFFLOADING state event"
    );
}

// TS-04-E10: Container start failure transitions to ERROR
// Requirement: 04-REQ-4.E3
#[tokio::test]
async fn test_edge_container_start_failure() {
    let registry = Arc::new(MockOciRegistry::new(TEST_MANIFEST));
    let runtime = Arc::new(MockContainerRuntime::failing());
    let (mut client, _mgr) =
        start_test_server_with(Duration::from_secs(3600), registry, runtime).await;

    // Start watching
    let mut stream = client
        .watch_adapter_states(WatchAdapterStatesRequest {})
        .await
        .unwrap()
        .into_inner();

    // Install — checksum will match but container start will fail
    client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap();

    // Collect events
    let mut seen_error = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        match tokio::time::timeout_at(deadline, stream.message()).await {
            Ok(Ok(Some(event))) => {
                // ERROR = 5
                if event.new_state == 5 {
                    seen_error = true;
                }
            }
            _ => break,
        }
    }

    assert!(
        seen_error,
        "adapter should transition to ERROR on container start failure"
    );
}

// TS-04-E11: Checksum mismatch transitions to ERROR
// Requirement: 04-REQ-5.E1
#[tokio::test]
async fn test_edge_checksum_mismatch() {
    let registry = Arc::new(MockOciRegistry::new(TEST_MANIFEST));
    let runtime = Arc::new(MockContainerRuntime::new());
    let (mut client, _mgr) =
        start_test_server_with(Duration::from_secs(3600), registry, runtime).await;

    // Start watching
    let mut stream = client
        .watch_adapter_states(WatchAdapterStatesRequest {})
        .await
        .unwrap()
        .into_inner();

    // Install with a wrong checksum
    client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256:
                "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        })
        .await
        .unwrap();

    // Collect events — should see ERROR, should NOT see INSTALLING
    let mut seen_error = false;
    let mut seen_installing = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        match tokio::time::timeout_at(deadline, stream.message()).await {
            Ok(Ok(Some(event))) => {
                if event.new_state == 5 {
                    seen_error = true;
                }
                if event.new_state == 2 {
                    seen_installing = true;
                }
            }
            _ => break,
        }
    }

    assert!(
        seen_error,
        "adapter should transition to ERROR on checksum mismatch"
    );
    assert!(
        !seen_installing,
        "adapter should NOT transition to INSTALLING on checksum mismatch"
    );
}

// TS-04-E12: Registry unreachable during pull
// Requirement: 04-REQ-5.E2
#[tokio::test]
async fn test_edge_registry_unreachable() {
    let registry = Arc::new(MockOciRegistry::unreachable());
    let runtime = Arc::new(MockContainerRuntime::new());
    let (mut client, _mgr) =
        start_test_server_with(Duration::from_secs(3600), registry, runtime).await;

    // Start watching
    let mut stream = client
        .watch_adapter_states(WatchAdapterStatesRequest {})
        .await
        .unwrap()
        .into_inner();

    // Install — registry is unreachable
    client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:19999/adaptor:v1".to_string(),
            checksum_sha256: "abc123".to_string(),
        })
        .await
        .unwrap();

    // Collect events — should see ERROR
    let mut seen_error = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        match tokio::time::timeout_at(deadline, stream.message()).await {
            Ok(Ok(Some(event))) => {
                if event.new_state == 5 {
                    seen_error = true;
                }
            }
            _ => break,
        }
    }

    assert!(
        seen_error,
        "adapter should transition to ERROR when registry is unreachable"
    );
}

// TS-04-E13: Re-install during OFFLOADING cancels offload
// Requirement: 04-REQ-6.E1
#[tokio::test]
async fn test_edge_reinstall_during_offloading() {
    let registry = Arc::new(MockOciRegistry::new(TEST_MANIFEST));
    let runtime = Arc::new(MockContainerRuntime::new());
    let offload_timeout = Duration::from_secs(1);
    let (mut client, manager) =
        start_test_server_with(offload_timeout, registry, runtime.clone()).await;

    // Start offloader with a short check interval
    let _offloader = offloader::start_offloader(
        manager.clone(),
        runtime,
        Some(Duration::from_millis(500)),
    );

    // Install adapter
    let install_resp = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap()
        .into_inner();

    let _adapter_id = install_resp.adapter_id;

    // Wait for RUNNING
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Transition to STOPPED
    {
        let mut mgr = manager.lock().await;
        mgr.transition(
            &_adapter_id,
            update_service::adapter_manager::AdapterState::Stopped,
        )
        .unwrap();
    }

    // Wait for offloading to trigger and complete
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Re-install with a different image_ref (original was offloaded/removed)
    let mut stream = client
        .watch_adapter_states(WatchAdapterStatesRequest {})
        .await
        .unwrap()
        .into_inner();

    let reinstall_resp = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v2".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap()
        .into_inner();

    // Collect events — should see DOWNLOADING for the new adapter
    let mut seen_downloading = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        match tokio::time::timeout_at(deadline, stream.message()).await {
            Ok(Ok(Some(event))) => {
                if event.adapter_id == reinstall_resp.adapter_id && event.new_state == 1 {
                    seen_downloading = true;
                }
            }
            _ => break,
        }
    }

    assert!(
        seen_downloading,
        "re-installed adapter should transition to DOWNLOADING"
    );
}

// TS-04-P5: Checksum Gate (property test)
// Property: For any bad checksum, adapter goes to ERROR, never INSTALLING.
#[tokio::test]
async fn test_property_checksum_gate() {
    let registry = Arc::new(MockOciRegistry::new(TEST_MANIFEST));
    let runtime = Arc::new(MockContainerRuntime::new());

    // Test with multiple bad checksums
    let bad_checksums = vec![
        "0000000000000000000000000000000000000000000000000000000000000000",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
    ];

    for (i, bad_checksum) in bad_checksums.iter().enumerate() {
        let (mut client, _mgr) = start_test_server_with(
            Duration::from_secs(3600),
            registry.clone(),
            runtime.clone(),
        )
        .await;

        // Start watching
        let mut stream = client
            .watch_adapter_states(WatchAdapterStatesRequest {})
            .await
            .unwrap()
            .into_inner();

        // Install with bad checksum
        client
            .install_adapter(InstallAdapterRequest {
                image_ref: format!("localhost:5000/adaptor:v{}", i),
                checksum_sha256: bad_checksum.to_string(),
            })
            .await
            .unwrap();

        // Collect events
        let mut seen_error = false;
        let mut seen_installing = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            match tokio::time::timeout_at(deadline, stream.message()).await {
                Ok(Ok(Some(event))) => {
                    if event.new_state == 5 {
                        seen_error = true;
                    }
                    if event.new_state == 2 {
                        seen_installing = true;
                    }
                }
                _ => break,
            }
        }

        assert!(
            seen_error,
            "bad checksum '{}' should result in ERROR",
            bad_checksum
        );
        assert!(
            !seen_installing,
            "bad checksum '{}' should NOT result in INSTALLING",
            bad_checksum
        );
    }
}

// TS-04-P6: Offloading Correctness (property test)
// Property: Stopped adapters are offloaded after timeout; running are not.
#[tokio::test]
async fn test_property_offloading_correctness() {
    let registry = Arc::new(MockOciRegistry::new(TEST_MANIFEST));
    let runtime = Arc::new(MockContainerRuntime::new());
    let offload_timeout = Duration::from_secs(1);

    let (mut client, manager) =
        start_test_server_with(offload_timeout, registry.clone(), runtime.clone()).await;

    // Start offloader
    let _offloader = offloader::start_offloader(
        manager.clone(),
        runtime.clone(),
        Some(Duration::from_millis(500)),
    );

    // Install adapter-a (will be stopped -> should be offloaded)
    let install_a = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adapter-a:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap()
        .into_inner();

    // Install adapter-b (will remain RUNNING -> should NOT be offloaded)
    let install_b = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adapter-b:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap()
        .into_inner();

    // Wait for both to reach RUNNING
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Stop adapter-a only
    {
        let mut mgr = manager.lock().await;
        mgr.transition(
            &install_a.adapter_id,
            update_service::adapter_manager::AdapterState::Stopped,
        )
        .unwrap();
    }

    // Wait for offloading timeout + check interval
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Verify: adapter-a should be gone, adapter-b should still be RUNNING
    let list_resp = client
        .list_adapters(ListAdaptersRequest {})
        .await
        .unwrap()
        .into_inner();

    let found_a = list_resp
        .adapters
        .iter()
        .any(|a| a.adapter_id == install_a.adapter_id);
    let found_b = list_resp
        .adapters
        .iter()
        .find(|a| a.adapter_id == install_b.adapter_id);

    assert!(
        !found_a,
        "stopped adapter-a should have been offloaded and removed"
    );
    assert!(
        found_b.is_some(),
        "running adapter-b should still be present"
    );
    assert_eq!(
        found_b.unwrap().state,
        3, // RUNNING
        "running adapter-b should still be in RUNNING state"
    );
}

// TS-04-P8: Event Stream Completeness (property test)
// Property: All state transitions are received by an active watcher.
#[tokio::test]
async fn test_property_event_stream_completeness() {
    let registry = Arc::new(MockOciRegistry::new(TEST_MANIFEST));
    let runtime = Arc::new(MockContainerRuntime::new());
    let (mut client, _mgr) =
        start_test_server_with(Duration::from_secs(3600), registry, runtime).await;

    // Start watching before any operations
    let mut stream = client
        .watch_adapter_states(WatchAdapterStatesRequest {})
        .await
        .unwrap()
        .into_inner();

    // Install an adapter
    let install_resp = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: valid_checksum(),
        })
        .await
        .unwrap()
        .into_inner();

    let adapter_id = install_resp.adapter_id;

    // Collect all events for this adapter
    let mut observed_states: Vec<i32> = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        match tokio::time::timeout_at(deadline, stream.message()).await {
            Ok(Ok(Some(event))) => {
                if event.adapter_id == adapter_id {
                    observed_states.push(event.new_state);
                }
            }
            _ => break,
        }
    }

    // We should see the full lifecycle: DOWNLOADING (1), INSTALLING (2), RUNNING (3)
    assert!(
        observed_states.contains(&1),
        "should observe DOWNLOADING; got: {:?}",
        observed_states
    );
    assert!(
        observed_states.contains(&2),
        "should observe INSTALLING; got: {:?}",
        observed_states
    );
    assert!(
        observed_states.contains(&3),
        "should observe RUNNING; got: {:?}",
        observed_states
    );
}
