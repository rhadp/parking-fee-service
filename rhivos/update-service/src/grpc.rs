use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};
use crate::podman::PodmanExecutor;
use crate::proto;
use crate::state::StateManager;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

/// Response from the install_adapter operation.
#[derive(Debug)]
pub struct InstallResponse {
    pub job_id: String,
    pub adapter_id: String,
    pub state: AdapterState,
}

/// Service-layer errors mapped to gRPC status codes.
#[derive(Debug)]
pub enum ServiceError {
    InvalidArgument(String),
    NotFound(String),
    Internal(String),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            Self::NotFound(msg) => write!(f, "not found: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for ServiceError {}

impl From<ServiceError> for tonic::Status {
    fn from(err: ServiceError) -> Self {
        match err {
            ServiceError::InvalidArgument(msg) => tonic::Status::invalid_argument(msg),
            ServiceError::NotFound(msg) => tonic::Status::not_found(msg),
            ServiceError::Internal(msg) => tonic::Status::internal(msg),
        }
    }
}

// ---------------------------------------------------------------------------
// Type conversions: internal <-> proto
// ---------------------------------------------------------------------------

fn adapter_state_to_proto(state: AdapterState) -> i32 {
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

fn event_to_proto(event: AdapterStateEvent) -> proto::AdapterStateEvent {
    proto::AdapterStateEvent {
        adapter_id: event.adapter_id,
        old_state: adapter_state_to_proto(event.old_state),
        new_state: adapter_state_to_proto(event.new_state),
        timestamp: event.timestamp,
    }
}

// ---------------------------------------------------------------------------
// Core service implementation (business logic, testable without gRPC)
// ---------------------------------------------------------------------------

/// Core service implementation, parameterized by a PodmanExecutor for testability.
pub struct UpdateServiceImpl<P: PodmanExecutor> {
    pub(crate) state_manager: Arc<StateManager>,
    pub(crate) podman: Arc<P>,
    pub(crate) broadcaster: broadcast::Sender<AdapterStateEvent>,
}

impl<P: PodmanExecutor + 'static> UpdateServiceImpl<P> {
    pub fn new(
        state_manager: Arc<StateManager>,
        podman: Arc<P>,
        broadcaster: broadcast::Sender<AdapterStateEvent>,
    ) -> Self {
        Self {
            state_manager,
            podman,
            broadcaster,
        }
    }

    /// Install an adapter from an OCI image reference.
    /// Returns immediately with DOWNLOADING state; actual pull/run happens asynchronously.
    pub async fn install_adapter(
        &self,
        image_ref: &str,
        checksum_sha256: &str,
    ) -> Result<InstallResponse, ServiceError> {
        // Validate inputs (REQ-1.E1, REQ-1.E2).
        if image_ref.is_empty() {
            return Err(ServiceError::InvalidArgument(
                "image_ref is required".to_string(),
            ));
        }
        if checksum_sha256.is_empty() {
            return Err(ServiceError::InvalidArgument(
                "checksum_sha256 is required".to_string(),
            ));
        }

        let adapter_id = crate::adapter::derive_adapter_id(image_ref);
        let job_id = uuid::Uuid::new_v4().to_string();

        // Create adapter entry with initial UNKNOWN state.
        let entry = AdapterEntry {
            adapter_id: adapter_id.clone(),
            image_ref: image_ref.to_string(),
            checksum_sha256: checksum_sha256.to_string(),
            state: AdapterState::Unknown,
            job_id: job_id.clone(),
            stopped_at: None,
            error_message: None,
        };
        self.state_manager.create_adapter(entry);

        // Transition to DOWNLOADING before returning.
        let _ = self
            .state_manager
            .transition(&adapter_id, AdapterState::Downloading, None);

        // Spawn async task for pull → verify → run pipeline.
        let state_mgr = self.state_manager.clone();
        let podman = self.podman.clone();
        let image = image_ref.to_string();
        let checksum = checksum_sha256.to_string();
        let aid = adapter_id.clone();

        tokio::spawn(async move {
            // Single-adapter constraint: stop any currently running adapter (REQ-2.1).
            if let Some(running) = state_mgr.get_running_adapter() {
                if running.adapter_id != aid {
                    match podman.stop(&running.adapter_id).await {
                        Ok(()) => {
                            let _ = state_mgr.transition(
                                &running.adapter_id,
                                AdapterState::Stopped,
                                None,
                            );
                        }
                        Err(e) => {
                            // REQ-2.E1: stop failure → old adapter ERROR, proceed anyway.
                            let _ = state_mgr.transition(
                                &running.adapter_id,
                                AdapterState::Error,
                                Some(e.message.clone()),
                            );
                        }
                    }
                }
            }

            // Pull image (REQ-1.2).
            if let Err(e) = podman.pull(&image).await {
                let _ =
                    state_mgr.transition(&aid, AdapterState::Error, Some(e.message));
                return;
            }

            // Inspect digest and verify checksum (REQ-1.3).
            let digest = match podman.inspect_digest(&image).await {
                Ok(d) => d,
                Err(e) => {
                    let _ = state_mgr.transition(
                        &aid,
                        AdapterState::Error,
                        Some(e.message),
                    );
                    return;
                }
            };

            if digest.trim() != checksum {
                // REQ-1.E4: checksum mismatch → ERROR, remove image.
                let _ = state_mgr.transition(
                    &aid,
                    AdapterState::Error,
                    Some("checksum_mismatch".to_string()),
                );
                let _ = podman.rmi(&image).await;
                return;
            }

            // Transition to INSTALLING (REQ-1.4).
            let _ = state_mgr.transition(&aid, AdapterState::Installing, None);

            // Run container (REQ-1.4).
            if let Err(e) = podman.run(&aid, &image).await {
                let _ =
                    state_mgr.transition(&aid, AdapterState::Error, Some(e.message));
                return;
            }

            // Transition to RUNNING (REQ-1.5).
            let _ = state_mgr.transition(&aid, AdapterState::Running, None);

            // Monitor container exit (REQ-9.1, REQ-9.2, REQ-9.E1).
            // This awaits `podman wait`, so the spawned task stays alive
            // until the container exits. The guard inside monitor_container
            // prevents races with explicit stop/remove operations.
            crate::monitor::monitor_container(&aid, state_mgr, podman).await;
        });

        Ok(InstallResponse {
            job_id,
            adapter_id,
            state: AdapterState::Downloading,
        })
    }

    /// Remove an adapter (stop if running, remove container and image).
    pub async fn remove_adapter(&self, adapter_id: &str) -> Result<(), ServiceError> {
        let entry = self
            .state_manager
            .get_adapter(adapter_id)
            .ok_or_else(|| ServiceError::NotFound("adapter not found".to_string()))?;

        // Stop if currently running (REQ-5.1, REQ-8.1).
        if entry.state == AdapterState::Running {
            match self.podman.stop(adapter_id).await {
                Ok(()) => {
                    // Emit RUNNING→STOPPED event as required by REQ-8.1 ("every
                    // state transition"). The container monitor guard will see the
                    // adapter is no longer RUNNING and skip its own transition.
                    let _ = self.state_manager.transition(
                        adapter_id,
                        AdapterState::Stopped,
                        None,
                    );
                }
                Err(e) => {
                    let _ = self.state_manager.transition(
                        adapter_id,
                        AdapterState::Error,
                        Some(e.message.clone()),
                    );
                }
            }
        }

        // Remove container (REQ-5.1).
        if let Err(e) = self.podman.rm(adapter_id).await {
            let _ = self.state_manager.transition(
                adapter_id,
                AdapterState::Error,
                Some(e.message.clone()),
            );
            return Err(ServiceError::Internal(e.message));
        }

        // Remove image (REQ-5.1).
        if let Err(e) = self.podman.rmi(&entry.image_ref).await {
            let _ = self.state_manager.transition(
                adapter_id,
                AdapterState::Error,
                Some(e.message.clone()),
            );
            return Err(ServiceError::Internal(e.message));
        }

        // Remove from in-memory state (REQ-5.2).
        self.state_manager
            .remove_adapter(adapter_id)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(())
    }

    /// Get the current status of an adapter.
    pub async fn get_adapter_status(
        &self,
        adapter_id: &str,
    ) -> Result<AdapterEntry, ServiceError> {
        self.state_manager
            .get_adapter(adapter_id)
            .ok_or_else(|| ServiceError::NotFound("adapter not found".to_string()))
    }

    /// List all known adapters.
    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        self.state_manager.list_adapters()
    }

    /// Subscribe to adapter state events.
    pub fn watch_adapter_states(&self) -> broadcast::Receiver<AdapterStateEvent> {
        self.broadcaster.subscribe()
    }
}

// ---------------------------------------------------------------------------
// Tonic gRPC trait implementation
// ---------------------------------------------------------------------------

/// Wrapper that implements the generated tonic `UpdateService` trait,
/// delegating to `UpdateServiceImpl` for business logic.
pub struct GrpcUpdateService {
    inner: UpdateServiceImpl<crate::podman::RealPodmanExecutor>,
}

impl GrpcUpdateService {
    pub fn new(
        state_manager: Arc<StateManager>,
        podman: Arc<crate::podman::RealPodmanExecutor>,
        broadcaster: broadcast::Sender<AdapterStateEvent>,
    ) -> Self {
        Self {
            inner: UpdateServiceImpl::new(state_manager, podman, broadcaster),
        }
    }
}

type WatchStream = Pin<
    Box<dyn tokio_stream::Stream<Item = Result<proto::AdapterStateEvent, tonic::Status>> + Send>,
>;

#[tonic::async_trait]
impl proto::update_service_server::UpdateService for GrpcUpdateService {
    async fn install_adapter(
        &self,
        request: tonic::Request<proto::InstallAdapterRequest>,
    ) -> Result<tonic::Response<proto::InstallAdapterResponse>, tonic::Status> {
        let req = request.into_inner();
        let resp = self
            .inner
            .install_adapter(&req.image_ref, &req.checksum_sha256)
            .await
            .map_err(tonic::Status::from)?;

        Ok(tonic::Response::new(proto::InstallAdapterResponse {
            job_id: resp.job_id,
            adapter_id: resp.adapter_id,
            state: adapter_state_to_proto(resp.state),
        }))
    }

    type WatchAdapterStatesStream = WatchStream;

    async fn watch_adapter_states(
        &self,
        _request: tonic::Request<proto::WatchAdapterStatesRequest>,
    ) -> Result<tonic::Response<Self::WatchAdapterStatesStream>, tonic::Status> {
        let rx = self.inner.watch_adapter_states();
        let stream = BroadcastStream::new(rx).filter_map(|result| match result {
            Ok(event) => Some(Ok(event_to_proto(event))),
            Err(_) => None, // Skip lagged messages
        });
        Ok(tonic::Response::new(Box::pin(stream)))
    }

    async fn list_adapters(
        &self,
        _request: tonic::Request<proto::ListAdaptersRequest>,
    ) -> Result<tonic::Response<proto::ListAdaptersResponse>, tonic::Status> {
        let adapters = self.inner.list_adapters();
        let proto_adapters = adapters
            .into_iter()
            .map(|a| proto::AdapterInfo {
                adapter_id: a.adapter_id,
                state: adapter_state_to_proto(a.state),
                image_ref: a.image_ref,
            })
            .collect();

        Ok(tonic::Response::new(proto::ListAdaptersResponse {
            adapters: proto_adapters,
        }))
    }

    async fn remove_adapter(
        &self,
        request: tonic::Request<proto::RemoveAdapterRequest>,
    ) -> Result<tonic::Response<proto::RemoveAdapterResponse>, tonic::Status> {
        let req = request.into_inner();
        self.inner
            .remove_adapter(&req.adapter_id)
            .await
            .map_err(tonic::Status::from)?;
        Ok(tonic::Response::new(proto::RemoveAdapterResponse {}))
    }

    async fn get_adapter_status(
        &self,
        request: tonic::Request<proto::GetAdapterStatusRequest>,
    ) -> Result<tonic::Response<proto::GetAdapterStatusResponse>, tonic::Status> {
        let req = request.into_inner();
        let entry = self
            .inner
            .get_adapter_status(&req.adapter_id)
            .await
            .map_err(tonic::Status::from)?;

        Ok(tonic::Response::new(proto::GetAdapterStatusResponse {
            adapter_id: entry.adapter_id,
            state: adapter_state_to_proto(entry.state),
            image_ref: entry.image_ref,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::AdapterState;
    use crate::podman::MockPodmanExecutor;
    use crate::state::StateManager;

    // -- REQ-8.1 / remove_adapter: emits RUNNING→STOPPED event on success ---
    // Validates the major review finding: RemoveAdapter must emit a state
    // event for the RUNNING→STOPPED transition when it stops a running adapter.

    #[tokio::test]
    async fn test_remove_running_adapter_emits_stopped_event() {
        let mock = Arc::new(MockPodmanExecutor::new());
        // Install pipeline: pull → inspect → run (adapter reaches RUNNING)
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc".to_string()));
        mock.set_run_result(Ok(()));
        // Removal pipeline: stop → rm → rmi all succeed
        mock.set_stop_result(Ok(()));
        mock.set_rm_result(Ok(()));
        mock.set_rmi_result(Ok(()));

        let (tx, _rx) = broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx.clone()));
        let service = UpdateServiceImpl::new(state_mgr.clone(), mock, tx.clone());

        // Install and wait for adapter to reach RUNNING.
        service
            .install_adapter("example.com/adapter:v1", "sha256:abc")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert_eq!(
            state_mgr.get_adapter("adapter-v1").unwrap().state,
            AdapterState::Running,
            "adapter should be RUNNING before removal"
        );

        // Subscribe to events AFTER install (no historical replay).
        let mut rx = tx.subscribe();

        // Remove the adapter.
        service.remove_adapter("adapter-v1").await.unwrap();

        // Collect events emitted during removal.
        let mut events = Vec::new();
        loop {
            match tokio::time::timeout(
                std::time::Duration::from_millis(100),
                rx.recv(),
            )
            .await
            {
                Ok(Ok(event)) => events.push(event),
                _ => break,
            }
        }

        // REQ-8.1: a RUNNING→STOPPED event MUST be emitted.
        let stopped_event = events.iter().find(|e| {
            e.old_state == AdapterState::Running && e.new_state == AdapterState::Stopped
        });
        assert!(
            stopped_event.is_some(),
            "RemoveAdapter must emit RUNNING→STOPPED event per REQ-8.1, got: {:?}",
            events
                .iter()
                .map(|e| (e.old_state, e.new_state))
                .collect::<Vec<_>>()
        );
    }

    // -- TS-07-E11: Podman removal failure returns INTERNAL -----------------

    #[tokio::test]
    async fn test_removal_failure_internal() {
        let mock = Arc::new(MockPodmanExecutor::new());
        // Set up an adapter in stopped state (via install + stop sequence)
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok("sha256:abc".to_string()));
        mock.set_run_result(Ok(()));
        // rm will fail during removal
        mock.set_stop_result(Ok(()));
        mock.set_rm_result(Err(crate::podman::PodmanError::new("container in use")));

        let (tx, _rx) = broadcast::channel(64);
        let state_mgr = Arc::new(StateManager::new(tx.clone()));
        let service = UpdateServiceImpl::new(state_mgr.clone(), mock, tx);

        // Install adapter first
        service
            .install_adapter("example.com/adapter:v1", "sha256:abc")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Attempt removal (rm will fail)
        let result = service.remove_adapter("adapter-v1").await;
        assert!(result.is_err(), "remove should fail when podman rm fails");
        let err = result.unwrap_err();
        assert!(
            matches!(err, ServiceError::Internal(_)),
            "error should be Internal, got {err:?}"
        );

        // Adapter should be in ERROR state
        let adapter = state_mgr.get_adapter("adapter-v1");
        assert!(adapter.is_some());
        assert_eq!(adapter.unwrap().state, crate::adapter::AdapterState::Error);
    }
}
