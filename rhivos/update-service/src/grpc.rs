use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};

use crate::adapter::{derive_adapter_id, AdapterEntry, AdapterState, AdapterStateEvent};
use crate::monitor::monitor_container;
use crate::podman::PodmanExecutor;
use crate::state::StateManager;

pub mod proto {
    tonic::include_proto!("update_service.v1");
}

/// The gRPC service implementation for UpdateService.
pub struct UpdateServiceImpl {
    pub state_mgr: Arc<StateManager>,
    pub podman: Arc<dyn PodmanExecutor>,
    pub broadcaster: broadcast::Sender<AdapterStateEvent>,
}

impl UpdateServiceImpl {
    pub fn new(
        state_mgr: Arc<StateManager>,
        podman: Arc<dyn PodmanExecutor>,
        broadcaster: broadcast::Sender<AdapterStateEvent>,
    ) -> Self {
        Self {
            state_mgr,
            podman,
            broadcaster,
        }
    }

    /// Runs the async install workflow (pull, verify, run).
    /// Called from a spawned task after the initial response is returned.
    pub async fn install_adapter_workflow(
        state_mgr: Arc<StateManager>,
        podman: Arc<dyn PodmanExecutor>,
        adapter_id: String,
        image_ref: String,
        checksum_sha256: String,
    ) {
        // Step 1: Pull the image
        if let Err(e) = podman.pull(&image_ref).await {
            let _ = state_mgr.transition(&adapter_id, AdapterState::Error, Some(e.message));
            return;
        }

        // Step 2: Inspect digest and verify checksum
        match podman.inspect_digest(&image_ref).await {
            Ok(digest) => {
                if digest.trim() != checksum_sha256.trim() {
                    // Checksum mismatch: transition to ERROR and remove image
                    let _ = state_mgr.transition(
                        &adapter_id,
                        AdapterState::Error,
                        Some("checksum_mismatch".to_string()),
                    );
                    let _ = podman.rmi(&image_ref).await;
                    return;
                }
            }
            Err(e) => {
                let _ = state_mgr.transition(&adapter_id, AdapterState::Error, Some(e.message));
                return;
            }
        }

        // Step 3: Transition to INSTALLING
        if let Err(e) = state_mgr.transition(&adapter_id, AdapterState::Installing, None) {
            tracing::error!("failed to transition to INSTALLING: {e}");
            return;
        }

        // Step 4: Run the container
        if let Err(e) = podman.run(&adapter_id, &image_ref).await {
            let _ = state_mgr.transition(&adapter_id, AdapterState::Error, Some(e.message));
            return;
        }

        // Step 5: Transition to RUNNING
        if let Err(e) = state_mgr.transition(&adapter_id, AdapterState::Running, None) {
            tracing::error!("failed to transition to RUNNING: {e}");
            return;
        }

        // Step 6: Spawn container monitor for exit detection.
        // The monitor calls `podman wait` and transitions the adapter to
        // STOPPED (exit code 0) or ERROR (non-zero / wait failure).
        tokio::spawn(monitor_container(state_mgr, podman, adapter_id));
    }
}

/// Convert internal AdapterState to proto enum value.
fn adapter_state_to_proto(state: &AdapterState) -> i32 {
    match state {
        AdapterState::Unknown => proto::AdapterState::Unknown as i32,
        AdapterState::Downloading => proto::AdapterState::Downloading as i32,
        AdapterState::Installing => proto::AdapterState::Installing as i32,
        AdapterState::Running => proto::AdapterState::Running as i32,
        AdapterState::Stopped => proto::AdapterState::Stopped as i32,
        AdapterState::Error => proto::AdapterState::Error as i32,
        AdapterState::Offloading => proto::AdapterState::Offloading as i32,
    }
}

type EventStream =
    Pin<Box<dyn futures::Stream<Item = Result<proto::AdapterStateEvent, Status>> + Send>>;

#[tonic::async_trait]
impl proto::update_service_server::UpdateService for UpdateServiceImpl {
    async fn install_adapter(
        &self,
        request: Request<proto::InstallAdapterRequest>,
    ) -> Result<Response<proto::InstallAdapterResponse>, Status> {
        let req = request.into_inner();

        // Validate inputs
        if req.image_ref.is_empty() {
            return Err(Status::invalid_argument("image_ref is required"));
        }
        if req.checksum_sha256.is_empty() {
            return Err(Status::invalid_argument("checksum_sha256 is required"));
        }

        // Derive adapter ID
        let adapter_id = derive_adapter_id(&req.image_ref);
        let job_id = uuid::Uuid::new_v4().to_string();

        // Single adapter constraint: stop any currently running adapter
        if let Some(running) = self.state_mgr.get_running_adapter() {
            match self.podman.stop(&running.adapter_id).await {
                Ok(()) => {
                    let _ = self.state_mgr.transition(
                        &running.adapter_id,
                        AdapterState::Stopped,
                        None,
                    );
                }
                Err(e) => {
                    // Stop failed: transition old adapter to ERROR, but proceed
                    let _ = self.state_mgr.transition(
                        &running.adapter_id,
                        AdapterState::Error,
                        Some(e.message),
                    );
                }
            }
        }

        // Create adapter entry in UNKNOWN state
        let entry = AdapterEntry {
            adapter_id: adapter_id.clone(),
            image_ref: req.image_ref.clone(),
            checksum_sha256: req.checksum_sha256.clone(),
            state: AdapterState::Unknown,
            job_id: job_id.clone(),
            stopped_at: None,
            error_message: None,
        };
        self.state_mgr.create_adapter(entry);

        // Transition to DOWNLOADING
        let _ = self
            .state_mgr
            .transition(&adapter_id, AdapterState::Downloading, None);

        // Spawn the async install workflow
        let state_mgr = self.state_mgr.clone();
        let podman = self.podman.clone();
        let aid = adapter_id.clone();
        let iref = req.image_ref.clone();
        let csum = req.checksum_sha256.clone();
        tokio::spawn(async move {
            Self::install_adapter_workflow(state_mgr, podman, aid, iref, csum).await;
        });

        Ok(Response::new(proto::InstallAdapterResponse {
            job_id,
            adapter_id,
            state: proto::AdapterState::Downloading as i32,
        }))
    }

    type WatchAdapterStatesStream = EventStream;

    async fn watch_adapter_states(
        &self,
        _request: Request<proto::WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        let rx = self.broadcaster.subscribe();
        let stream = BroadcastStream::new(rx).filter_map(|result| match result {
            Ok(event) => Some(Ok(proto::AdapterStateEvent {
                adapter_id: event.adapter_id,
                old_state: adapter_state_to_proto(&event.old_state),
                new_state: adapter_state_to_proto(&event.new_state),
                timestamp: event.timestamp as i64,
            })),
            Err(_) => None, // Skip lagged or closed errors
        });

        Ok(Response::new(Box::pin(stream)))
    }

    async fn list_adapters(
        &self,
        _request: Request<proto::ListAdaptersRequest>,
    ) -> Result<Response<proto::ListAdaptersResponse>, Status> {
        let adapters = self.state_mgr.list_adapters();
        let proto_adapters = adapters
            .iter()
            .map(|a| proto::AdapterInfo {
                adapter_id: a.adapter_id.clone(),
                state: adapter_state_to_proto(&a.state),
                image_ref: a.image_ref.clone(),
            })
            .collect();

        Ok(Response::new(proto::ListAdaptersResponse {
            adapters: proto_adapters,
        }))
    }

    async fn remove_adapter(
        &self,
        request: Request<proto::RemoveAdapterRequest>,
    ) -> Result<Response<proto::RemoveAdapterResponse>, Status> {
        let req = request.into_inner();
        let adapter_id = &req.adapter_id;

        // Look up the adapter
        let adapter = self
            .state_mgr
            .get_adapter(adapter_id)
            .ok_or_else(|| Status::not_found("adapter not found"))?;

        // Stop if running
        if adapter.state == AdapterState::Running {
            if let Err(e) = self.podman.stop(adapter_id).await {
                let _ = self.state_mgr.transition(
                    adapter_id,
                    AdapterState::Error,
                    Some(e.message.clone()),
                );
                return Err(Status::internal(format!("failed to stop adapter: {}", e.message)));
            }
        }

        // Remove container
        if let Err(e) = self.podman.rm(adapter_id).await {
            let _ =
                self.state_mgr
                    .transition(adapter_id, AdapterState::Error, Some(e.message.clone()));
            return Err(Status::internal(format!(
                "failed to remove container: {}",
                e.message
            )));
        }

        // Remove image
        if let Err(e) = self.podman.rmi(&adapter.image_ref).await {
            let _ =
                self.state_mgr
                    .transition(adapter_id, AdapterState::Error, Some(e.message.clone()));
            return Err(Status::internal(format!(
                "failed to remove image: {}",
                e.message
            )));
        }

        // Remove from state
        let _ = self.state_mgr.remove_adapter(adapter_id);

        Ok(Response::new(proto::RemoveAdapterResponse {
            adapter_id: adapter_id.to_string(),
            state: proto::AdapterState::Stopped as i32,
        }))
    }

    async fn get_adapter_status(
        &self,
        request: Request<proto::GetAdapterStatusRequest>,
    ) -> Result<Response<proto::GetAdapterStatusResponse>, Status> {
        let req = request.into_inner();

        let adapter = self
            .state_mgr
            .get_adapter(&req.adapter_id)
            .ok_or_else(|| Status::not_found("adapter not found"))?;

        Ok(Response::new(proto::GetAdapterStatusResponse {
            adapter: Some(proto::AdapterInfo {
                adapter_id: adapter.adapter_id,
                state: adapter_state_to_proto(&adapter.state),
                image_ref: adapter.image_ref,
            }),
        }))
    }
}
