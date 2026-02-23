//! UpdateService gRPC trait implementation.
//!
//! Implements the `UpdateService` gRPC service defined in
//! `update_service.proto`. Manages adapter lifecycle via `AdapterManager`,
//! supporting install, watch, list, remove, and status queries.
//!
//! Requirements covered:
//! - 04-REQ-4.1: gRPC service on configurable address
//! - 04-REQ-4.2: InstallAdapter returns job_id, adapter_id, state
//! - 04-REQ-4.3: WatchAdapterStates server-streaming
//! - 04-REQ-4.4: ListAdapters returns all adapters
//! - 04-REQ-4.5: RemoveAdapter stops and removes
//! - 04-REQ-4.6: GetAdapterStatus returns adapter info
//! - 04-REQ-4.E1: ALREADY_EXISTS for duplicate installs
//! - 04-REQ-4.E2: NOT_FOUND for unknown adapter IDs
//! - 04-REQ-5.1: OCI image pull on InstallAdapter
//! - 04-REQ-5.2: SHA-256 checksum verification
//! - 04-REQ-5.3: Checksum match -> INSTALLING transition
//! - 04-REQ-5.E1: Checksum mismatch -> ERROR state
//! - 04-REQ-5.E2: Registry unreachable -> ERROR state
//! - 04-REQ-4.E3: Container start failure -> ERROR state

use std::sync::Arc;

use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::adapter_manager::{AdapterManager, AdapterState};
use crate::container_runtime::ContainerRuntime;
use crate::oci_client::{OciRegistry, verify_manifest_checksum};
use crate::parking::common::v1 as common;
use crate::parking::update::v1::update_service_server::UpdateService;
use crate::parking::update::v1::{
    AdapterStateEvent, GetAdapterStatusRequest, GetAdapterStatusResponse, InstallAdapterRequest,
    InstallAdapterResponse, ListAdaptersRequest, ListAdaptersResponse, RemoveAdapterRequest,
    RemoveAdapterResponse, WatchAdapterStatesRequest,
};

/// Implementation of the UpdateService gRPC service.
///
/// Holds a shared, mutex-protected `AdapterManager` to coordinate
/// state across concurrent gRPC calls and background tasks, plus
/// an OCI registry client and container runtime for the async
/// install pipeline.
pub struct UpdateServiceGrpc<R: OciRegistry, C: ContainerRuntime> {
    manager: Arc<Mutex<AdapterManager>>,
    registry: Arc<R>,
    runtime: Arc<C>,
}

impl<R: OciRegistry, C: ContainerRuntime> UpdateServiceGrpc<R, C> {
    /// Create a new UpdateServiceGrpc with the given AdapterManager,
    /// OCI registry client, and container runtime.
    pub fn new(
        manager: Arc<Mutex<AdapterManager>>,
        registry: Arc<R>,
        runtime: Arc<C>,
    ) -> Self {
        UpdateServiceGrpc {
            manager,
            registry,
            runtime,
        }
    }
}

/// Async background task that drives the adapter through:
/// DOWNLOADING -> (checksum check) -> INSTALLING -> RUNNING or ERROR.
///
/// This is spawned by `install_adapter` so the gRPC call returns
/// immediately with DOWNLOADING state.
async fn install_pipeline<R: OciRegistry, C: ContainerRuntime>(
    manager: Arc<Mutex<AdapterManager>>,
    registry: Arc<R>,
    runtime: Arc<C>,
    adapter_id: String,
    image_ref: String,
    checksum_sha256: String,
) {
    // Step 1: Pull the OCI manifest
    let manifest_bytes = match registry.pull_manifest(&image_ref).await {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!(
                "install_pipeline: OCI pull failed for adapter {}: {}",
                adapter_id, e
            );
            let mut mgr = manager.lock().await;
            let _ = mgr.transition(&adapter_id, AdapterState::Error);
            return;
        }
    };

    // Step 2: Verify SHA-256 checksum of manifest
    if let Err(e) = verify_manifest_checksum(&manifest_bytes, &checksum_sha256) {
        eprintln!(
            "install_pipeline: checksum verification failed for adapter {}: {}",
            adapter_id, e
        );
        let mut mgr = manager.lock().await;
        let _ = mgr.transition(&adapter_id, AdapterState::Error);
        return;
    }

    // Step 3: Transition to INSTALLING
    {
        let mut mgr = manager.lock().await;
        // Check the adapter is still in DOWNLOADING state (could have been removed)
        match mgr.get_adapter(&adapter_id) {
            Some(record) if record.state == AdapterState::Downloading => {}
            _ => return, // Adapter was removed or state changed
        }
        if let Err(e) = mgr.transition(&adapter_id, AdapterState::Installing) {
            eprintln!(
                "install_pipeline: failed to transition {} to INSTALLING: {}",
                adapter_id, e
            );
            return;
        }
    }

    // Step 4: Start the container via container runtime
    match runtime.start_container(&adapter_id, &image_ref).await {
        Ok(container_id) => {
            // Step 5: Transition to RUNNING
            let mut mgr = manager.lock().await;
            if let Some(record) = mgr.get_adapter(&adapter_id) {
                if record.state == AdapterState::Installing {
                    // Store the container_id
                    // We need mutable access to the record, so use the transition
                    // and then set the container_id
                    if let Err(e) = mgr.transition(&adapter_id, AdapterState::Running) {
                        eprintln!(
                            "install_pipeline: failed to transition {} to RUNNING: {}",
                            adapter_id, e
                        );
                    }
                    // Set the container_id on the record
                    mgr.set_container_id(&adapter_id, Some(container_id));
                }
            }
        }
        Err(e) => {
            // Container start failed -> ERROR (04-REQ-4.E3)
            eprintln!(
                "install_pipeline: container start failed for adapter {}: {}",
                adapter_id, e
            );
            let mut mgr = manager.lock().await;
            let _ = mgr.transition(&adapter_id, AdapterState::Error);
        }
    }
}

#[tonic::async_trait]
impl<R: OciRegistry, C: ContainerRuntime> UpdateService for UpdateServiceGrpc<R, C> {
    /// Install an adapter by initiating an async download.
    ///
    /// Returns immediately with DOWNLOADING state. The actual OCI pull,
    /// checksum verification, and container start happen asynchronously.
    ///
    /// Returns `ALREADY_EXISTS` if an adapter with the same `image_ref`
    /// is already installed (04-REQ-4.E1).
    async fn install_adapter(
        &self,
        request: Request<InstallAdapterRequest>,
    ) -> Result<Response<InstallAdapterResponse>, Status> {
        let req = request.into_inner();

        let job_id = Uuid::new_v4().to_string();
        let adapter_id = format!("adapter-{}", Uuid::new_v4());

        let image_ref = req.image_ref.clone();
        let checksum = req.checksum_sha256.clone();

        {
            let mut mgr = self.manager.lock().await;
            mgr.install_adapter(adapter_id.clone(), req.image_ref, req.checksum_sha256)
                .map_err(Status::already_exists)?;
        }

        // Spawn the async install pipeline
        tokio::spawn(install_pipeline(
            self.manager.clone(),
            self.registry.clone(),
            self.runtime.clone(),
            adapter_id.clone(),
            image_ref,
            checksum,
        ));

        Ok(Response::new(InstallAdapterResponse {
            job_id,
            adapter_id,
            state: common::AdapterState::Downloading.into(),
        }))
    }

    type WatchAdapterStatesStream =
        tokio_stream::wrappers::ReceiverStream<Result<AdapterStateEvent, Status>>;

    /// Watch adapter state transitions via a server-streaming response.
    ///
    /// Subscribes to the AdapterManager's broadcast channel and relays
    /// each `StateEvent` as an `AdapterStateEvent` message.
    async fn watch_adapter_states(
        &self,
        _request: Request<WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        let mgr = self.manager.lock().await;
        let mut broadcast_rx = mgr.subscribe();
        drop(mgr); // Release the lock before spawning

        let (tx, rx) = tokio::sync::mpsc::channel(128);

        tokio::spawn(async move {
            loop {
                match broadcast_rx.recv().await {
                    Ok(event) => {
                        let proto_event = AdapterStateEvent {
                            adapter_id: event.adapter_id,
                            old_state: event.old_state.to_proto(),
                            new_state: event.new_state.to_proto(),
                            timestamp: event.timestamp,
                        };
                        if tx.send(Ok(proto_event)).await.is_err() {
                            // Client disconnected
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        eprintln!(
                            "watch_adapter_states: lagged behind by {} events",
                            n
                        );
                        // Continue receiving
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }

    /// List all known adapters with their current states.
    async fn list_adapters(
        &self,
        _request: Request<ListAdaptersRequest>,
    ) -> Result<Response<ListAdaptersResponse>, Status> {
        let mgr = self.manager.lock().await;
        let adapters = mgr
            .list_adapters()
            .iter()
            .map(|record| common::AdapterInfo {
                adapter_id: record.adapter_id.clone(),
                operator_id: String::new(),
                image_ref: record.image_ref.clone(),
                checksum_sha256: record.checksum_sha256.clone(),
                state: record.state.to_proto(),
            })
            .collect();

        Ok(Response::new(ListAdaptersResponse { adapters }))
    }

    /// Remove an adapter, stopping it if necessary.
    ///
    /// Returns `NOT_FOUND` if the adapter_id does not exist (04-REQ-4.E2).
    async fn remove_adapter(
        &self,
        request: Request<RemoveAdapterRequest>,
    ) -> Result<Response<RemoveAdapterResponse>, Status> {
        let req = request.into_inner();

        let mut mgr = self.manager.lock().await;
        mgr.remove_adapter(&req.adapter_id)
            .map_err(Status::not_found)?;

        Ok(Response::new(RemoveAdapterResponse {}))
    }

    /// Get the status of a single adapter.
    ///
    /// Returns `NOT_FOUND` if the adapter_id does not exist (04-REQ-4.E2).
    async fn get_adapter_status(
        &self,
        request: Request<GetAdapterStatusRequest>,
    ) -> Result<Response<GetAdapterStatusResponse>, Status> {
        let req = request.into_inner();

        let mgr = self.manager.lock().await;
        let record = mgr
            .get_adapter(&req.adapter_id)
            .ok_or_else(|| Status::not_found(format!("adapter {} not found", req.adapter_id)))?;

        let info = common::AdapterInfo {
            adapter_id: record.adapter_id.clone(),
            operator_id: String::new(),
            image_ref: record.image_ref.clone(),
            checksum_sha256: record.checksum_sha256.clone(),
            state: record.state.to_proto(),
        };

        Ok(Response::new(GetAdapterStatusResponse {
            adapter: Some(info),
        }))
    }
}
