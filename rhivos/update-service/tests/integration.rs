//! Integration tests for the UPDATE_SERVICE.
//!
//! These tests verify the UPDATE_SERVICE gRPC interface.
//! Tests for task group 5 use in-process gRPC (no external process needed).
//! Tests for task group 6 (OCI, checksum gate, offloading) are still `#[ignore]`.
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
use update_service::grpc_service::UpdateServiceGrpc;
use update_service::parking::update::v1::update_service_server::UpdateServiceServer;
use update_service::parking::update::v1::update_service_client::UpdateServiceClient;
use update_service::parking::update::v1::{
    InstallAdapterRequest, ListAdaptersRequest, RemoveAdapterRequest,
    GetAdapterStatusRequest, WatchAdapterStatesRequest,
};

/// Start an in-process UPDATE_SERVICE gRPC server on an ephemeral port.
/// Returns the gRPC client and a handle to the server task.
async fn start_test_server() -> (UpdateServiceClient<Channel>, Arc<Mutex<AdapterManager>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let manager = Arc::new(Mutex::new(AdapterManager::new(Duration::from_secs(24 * 3600))));
    let service = UpdateServiceGrpc::new(manager.clone());

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

// ==========================================================================
// TS-04-15: UPDATE_SERVICE exposes gRPC service
// Requirement: 04-REQ-4.1
// ==========================================================================

#[tokio::test]
async fn test_update_service_grpc_service() {
    let (mut client, _mgr) = start_test_server().await;

    // Verify the server responds to ListAdapters
    let response = client
        .list_adapters(ListAdaptersRequest {})
        .await;

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
            checksum_sha256: "abc123def456".to_string(),
        })
        .await;

    assert!(response.is_ok());
    let resp = response.unwrap().into_inner();
    assert!(!resp.job_id.is_empty(), "job_id should not be empty");
    assert!(!resp.adapter_id.is_empty(), "adapter_id should not be empty");
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
            checksum_sha256: "abc123".to_string(),
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
            checksum_sha256: "abc123".to_string(),
        })
        .await
        .unwrap();

    // List adapters
    let response = client
        .list_adapters(ListAdaptersRequest {})
        .await
        .unwrap()
        .into_inner();

    assert!(response.adapters.len() >= 1, "should have at least one adapter");
    let adapter = &response.adapters[0];
    assert!(!adapter.adapter_id.is_empty(), "adapter_id should not be empty");
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
            checksum_sha256: "abc123".to_string(),
        })
        .await
        .unwrap()
        .into_inner();

    let adapter_id = install_resp.adapter_id;

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
            checksum_sha256: "abc123".to_string(),
        })
        .await
        .unwrap()
        .into_inner();

    let adapter_id = install_resp.adapter_id;

    // Get adapter status
    let status_resp = client
        .get_adapter_status(GetAdapterStatusRequest {
            adapter_id: adapter_id.clone(),
        })
        .await
        .unwrap()
        .into_inner();

    let adapter = status_resp.adapter.expect("adapter info should be present");
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
            checksum_sha256: "abc123".to_string(),
        })
        .await
        .unwrap();

    // Try to install again with the same image_ref
    let result = client
        .install_adapter(InstallAdapterRequest {
            image_ref: "localhost:5000/adaptor:v1".to_string(),
            checksum_sha256: "abc123".to_string(),
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
// Task Group 6 tests (still #[ignore] — to be implemented later)
// ==========================================================================

// TS-04-21: OCI image pull on InstallAdapter
// Requirement: 04-REQ-5.1
#[tokio::test]
#[ignore = "requires UPDATE_SERVICE + mock OCI registry running"]
async fn test_oci_image_pull() {
    todo!("TS-04-21: OCI image pull test not yet implemented")
}

// TS-04-23: Checksum match transitions to INSTALLING
// Requirement: 04-REQ-5.3
#[tokio::test]
#[ignore = "requires UPDATE_SERVICE + mock OCI registry running"]
async fn test_oci_checksum_match_transition() {
    todo!("TS-04-23: checksum match -> INSTALLING transition not yet implemented")
}

// TS-04-25: Stopped adapter offloaded after timeout
// Requirement: 04-REQ-6.2
#[tokio::test]
#[ignore = "requires UPDATE_SERVICE running with short timeout"]
async fn test_offloading_stopped_adapter_offloaded() {
    todo!("TS-04-25: stopped adapter offloaded after timeout not yet implemented")
}

// TS-04-26: Offloading emits state events
// Requirement: 04-REQ-6.3
#[tokio::test]
#[ignore = "requires UPDATE_SERVICE running with short timeout"]
async fn test_offloading_emits_state_events() {
    todo!("TS-04-26: offloading emits state events not yet implemented")
}

// TS-04-E10: Container start failure transitions to ERROR
// Requirement: 04-REQ-4.E3
#[tokio::test]
#[ignore = "requires UPDATE_SERVICE running with broken image"]
async fn test_edge_container_start_failure() {
    todo!("TS-04-E10: container start failure not yet implemented")
}

// TS-04-E11: Checksum mismatch transitions to ERROR
// Requirement: 04-REQ-5.E1
#[tokio::test]
#[ignore = "requires UPDATE_SERVICE + mock OCI registry running"]
async fn test_edge_checksum_mismatch() {
    todo!("TS-04-E11: checksum mismatch not yet implemented")
}

// TS-04-E12: Registry unreachable during pull
// Requirement: 04-REQ-5.E2
#[tokio::test]
#[ignore = "requires UPDATE_SERVICE running, registry unreachable"]
async fn test_edge_registry_unreachable() {
    todo!("TS-04-E12: registry unreachable not yet implemented")
}

// TS-04-E13: Re-install during OFFLOADING cancels offload
// Requirement: 04-REQ-6.E1
#[tokio::test]
#[ignore = "requires UPDATE_SERVICE running with short timeout"]
async fn test_edge_reinstall_during_offloading() {
    todo!("TS-04-E13: re-install during offloading not yet implemented")
}

// TS-04-P5: Checksum Gate (property test)
#[tokio::test]
#[ignore = "requires UPDATE_SERVICE + mock OCI registry running"]
async fn test_property_checksum_gate() {
    todo!("TS-04-P5: checksum gate property not yet implemented")
}

// TS-04-P6: Offloading Correctness (property test)
#[tokio::test]
#[ignore = "requires UPDATE_SERVICE running with short timeout"]
async fn test_property_offloading_correctness() {
    todo!("TS-04-P6: offloading correctness property not yet implemented")
}

// TS-04-P8: Event Stream Completeness (property test)
#[tokio::test]
#[ignore = "requires UPDATE_SERVICE running"]
async fn test_property_event_stream_completeness() {
    todo!("TS-04-P8: event stream completeness property not yet implemented")
}
