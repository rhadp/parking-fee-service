//! gRPC server implementation for the UpdateService.
//!
//! Implements the `UpdateService` gRPC interface from `update_service.proto`:
//!
//! - `InstallAdapter` — create and start a container via podman.
//! - `RemoveAdapter` — stop and remove a container.
//! - `ListAdapters` — list all adapters with current state.
//! - `GetAdapterStatus` — get a single adapter's info and state.
//! - `WatchAdapterStates` — server-streaming RPC for state events.
//!
//! # Requirements
//!
//! - 04-REQ-4.1: Expose gRPC server implementing UpdateService.
//! - 04-REQ-4.2: InstallAdapter returns job_id, adapter_id, and state.
//! - 04-REQ-4.3: WatchAdapterStates streams AdapterStateEvent.
//! - 04-REQ-4.4: ListAdapters returns all known adapters.
//! - 04-REQ-4.5: GetAdapterStatus returns info and state for one adapter.
//! - 04-REQ-3.1: Create and start container with podman.
//! - 04-REQ-3.2: Pass env vars to container.
//! - 04-REQ-3.3: Stop and remove container.
//! - 04-REQ-3.6: Reconcile persisted state with podman on startup.

use std::sync::Arc;

use tokio::sync::{broadcast, Mutex};
use tonic::{Request, Response, Status};
use tracing::{error, info, warn};

use parking_proto::services::update::update_service_server::UpdateService;
use parking_proto::services::update::{
    AdapterStateEvent, GetAdapterStatusRequest, GetAdapterStatusResponse,
    InstallAdapterRequest, InstallAdapterResponse, ListAdaptersRequest,
    ListAdaptersResponse, RemoveAdapterRequest, RemoveAdapterResponse,
    WatchAdapterStatesRequest,
};

use crate::offload::{OffloadCallback, OffloadManager};
use crate::podman::ContainerRuntime;
use crate::state::{AdapterConfig, AdapterEntry, AdapterState, AdapterStore};

/// Container name prefix for adapter containers.
const CONTAINER_PREFIX: &str = "poa-";

/// Podman network for adapter containers.
const CONTAINER_NETWORK: &str = "host";

// ---------------------------------------------------------------------------
// StateNotifier — broadcast channel for state events
// ---------------------------------------------------------------------------

/// Broadcasts adapter state transition events to WatchAdapterStates streams.
#[derive(Debug, Clone)]
pub struct StateNotifier {
    sender: broadcast::Sender<AdapterStateEvent>,
}

impl StateNotifier {
    /// Create a new notifier with the given channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Emit a state transition event.
    pub fn notify(&self, event: AdapterStateEvent) {
        // Ignore send errors (no active subscribers is fine)
        let _ = self.sender.send(event);
    }

    /// Subscribe to state events.
    pub fn subscribe(&self) -> broadcast::Receiver<AdapterStateEvent> {
        self.sender.subscribe()
    }
}

// ---------------------------------------------------------------------------
// UpdateServiceImpl
// ---------------------------------------------------------------------------

/// Real implementation of the UpdateService gRPC server.
///
/// This replaces the stub implementation from task group 5.
pub struct UpdateServiceImpl<R: ContainerRuntime + 'static> {
    /// Adapter state store (persisted).
    store: Arc<Mutex<AdapterStore>>,
    /// Container runtime (podman).
    runtime: Arc<R>,
    /// State event broadcaster.
    notifier: StateNotifier,
    /// Offload timer manager.
    offload_mgr: OffloadManager,
    /// Default adapter config (env vars for containers).
    default_config: AdapterConfig,
}

impl<R: ContainerRuntime + 'static> UpdateServiceImpl<R> {
    /// Create a new UpdateServiceImpl.
    pub fn new(
        store: Arc<Mutex<AdapterStore>>,
        runtime: Arc<R>,
        offload_mgr: OffloadManager,
        default_config: AdapterConfig,
    ) -> Self {
        Self {
            store,
            runtime,
            notifier: StateNotifier::new(64),
            offload_mgr,
            default_config,
        }
    }

    /// Get a reference to the state notifier (for use in main.rs).
    pub fn notifier(&self) -> &StateNotifier {
        &self.notifier
    }

    /// Generate a container name from an adapter_id.
    fn container_name(adapter_id: &str) -> String {
        format!("{}{}", CONTAINER_PREFIX, adapter_id)
    }

    /// Generate a unique adapter ID.
    ///
    /// Uses a combination of timestamp and atomic counter to avoid
    /// collisions when multiple adapters are installed in quick succession.
    fn generate_adapter_id() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{SystemTime, UNIX_EPOCH};

        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("adapter-{:x}-{}", ts, seq)
    }

    /// Get current Unix timestamp.
    fn now_timestamp() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    /// Emit a state transition event.
    fn emit_event(&self, adapter_id: &str, old_state: &AdapterState, new_state: &AdapterState) {
        let event = AdapterStateEvent {
            adapter_id: adapter_id.to_string(),
            old_state: old_state.to_proto(),
            new_state: new_state.to_proto(),
            timestamp: Self::now_timestamp(),
        };
        self.notifier.notify(event);
    }

    /// Reconcile persisted state with actual podman container state.
    ///
    /// Called on startup. For each adapter in RUNNING state, checks if the
    /// container is actually running. If not, transitions to ERROR.
    ///
    /// # Requirements
    ///
    /// - 04-REQ-3.6: Reconcile persisted state with podman on startup.
    pub async fn reconcile(&self) {
        let mut store = self.store.lock().await;
        let adapter_ids: Vec<String> = store
            .list()
            .iter()
            .filter(|e| e.state == AdapterState::Running)
            .map(|e| e.adapter_id.clone())
            .collect();

        for adapter_id in adapter_ids {
            let entry = match store.get(&adapter_id) {
                Some(e) => e,
                None => continue,
            };
            let container_name = entry.container_name.clone();

            match self.runtime.is_running(&container_name).await {
                Ok(true) => {
                    info!(adapter_id = %adapter_id, "reconcile: container is running");
                }
                Ok(false) => {
                    warn!(
                        adapter_id = %adapter_id,
                        container = %container_name,
                        "reconcile: container not running, transitioning to ERROR"
                    );
                    let old_state = AdapterState::Running;
                    let new_state =
                        AdapterState::Error("container not running after restart".to_string());
                    if let Err(e) = store.transition(&adapter_id, new_state.clone()) {
                        error!(adapter_id = %adapter_id, error = %e, "reconcile: transition failed");
                    } else {
                        self.emit_event(&adapter_id, &old_state, &new_state);
                    }
                }
                Err(e) => {
                    warn!(
                        adapter_id = %adapter_id,
                        error = %e,
                        "reconcile: failed to check container state"
                    );
                }
            }
        }
    }

    /// Start offload timer for an adapter. Used both from session-end
    /// notification and from startup reconciliation.
    pub fn start_offload_timer(&self, adapter_id: String) {
        let store = self.store.clone();
        let runtime = self.runtime.clone();
        let notifier = self.notifier.clone();

        let callback: OffloadCallback = Arc::new(move |id| {
            let store = store.clone();
            let runtime = runtime.clone();
            let notifier = notifier.clone();

            tokio::spawn(async move {
                info!(adapter_id = %id, "offload: starting container removal");

                let mut store = store.lock().await;

                // Transition Running → Offloading
                let entry = match store.get(&id) {
                    Some(e) => e,
                    None => {
                        warn!(adapter_id = %id, "offload: adapter not found");
                        return;
                    }
                };

                if entry.state != AdapterState::Running {
                    warn!(adapter_id = %id, state = %entry.state, "offload: adapter not in Running state");
                    return;
                }

                let container_name = entry.container_name.clone();

                let old_state = AdapterState::Running;
                match store.transition(&id, AdapterState::Offloading) {
                    Ok(_) => {
                        let event = AdapterStateEvent {
                            adapter_id: id.clone(),
                            old_state: old_state.to_proto(),
                            new_state: AdapterState::Offloading.to_proto(),
                            timestamp: Self::now_timestamp(),
                        };
                        notifier.notify(event);
                    }
                    Err(e) => {
                        error!(adapter_id = %id, error = %e, "offload: transition to Offloading failed");
                        return;
                    }
                }

                // Stop and remove container
                if let Err(e) = runtime.stop_and_remove(&container_name).await {
                    error!(adapter_id = %id, error = %e, "offload: failed to stop/remove container");
                }

                // Transition Offloading → Unknown
                match store.transition(&id, AdapterState::Unknown) {
                    Ok(_) => {
                        let event = AdapterStateEvent {
                            adapter_id: id.clone(),
                            old_state: AdapterState::Offloading.to_proto(),
                            new_state: AdapterState::Unknown.to_proto(),
                            timestamp: Self::now_timestamp(),
                        };
                        notifier.notify(event);
                    }
                    Err(e) => {
                        error!(adapter_id = %id, error = %e, "offload: transition to Unknown failed");
                    }
                }

                info!(adapter_id = %id, "offload: adapter removed");
            });
        });

        let mgr = self.offload_mgr.clone();
        tokio::spawn(async move {
            mgr.start_timer(adapter_id, callback).await;
        });
    }
}

#[tonic::async_trait]
impl<R: ContainerRuntime + 'static> UpdateService for UpdateServiceImpl<R> {
    /// Install an adapter: create and start a container.
    ///
    /// # Requirements
    ///
    /// - 04-REQ-3.1: Create and start container.
    /// - 04-REQ-3.2: Pass env vars to container.
    /// - 04-REQ-3.E1: Podman failure → ERROR state.
    /// - 04-REQ-3.E2: Already RUNNING → return existing info.
    /// - 04-REQ-3.E3: Image not found → ERROR state.
    /// - 04-REQ-4.2: Return job_id, adapter_id, state.
    async fn install_adapter(
        &self,
        request: Request<InstallAdapterRequest>,
    ) -> Result<Response<InstallAdapterResponse>, Status> {
        let req = request.into_inner();
        let image_ref = req.image_ref;
        let checksum = req.checksum;

        info!(image_ref = %image_ref, checksum = %checksum, "InstallAdapter request");

        // Check if adapter already running with this image (04-REQ-3.E2)
        {
            let store = self.store.lock().await;
            if let Some(existing) = store.find_running_by_image(&image_ref) {
                info!(
                    adapter_id = %existing.adapter_id,
                    "adapter already running for image"
                );
                return Ok(Response::new(InstallAdapterResponse {
                    job_id: format!("job-{}", existing.adapter_id),
                    adapter_id: existing.adapter_id.clone(),
                    state: existing.state.to_proto(),
                }));
            }
        }

        let adapter_id = Self::generate_adapter_id();
        let container_name = Self::container_name(&adapter_id);
        let job_id = format!("job-{}", adapter_id);

        // Create entry in Unknown state, then transition to Installing
        let entry = AdapterEntry {
            adapter_id: adapter_id.clone(),
            image_ref: image_ref.clone(),
            checksum,
            container_name: container_name.clone(),
            state: AdapterState::Unknown,
            config: self.default_config.clone(),
            installed_at: Some(Self::now_timestamp()),
            session_ended_at: None,
        };

        let mut store = self.store.lock().await;
        store.insert(entry).map_err(|e| {
            error!(error = %e, "failed to insert adapter entry");
            Status::internal(format!("failed to insert adapter: {}", e))
        })?;

        // Unknown → Installing
        store
            .transition(&adapter_id, AdapterState::Installing)
            .map_err(|e| {
                error!(error = %e, "failed to transition to Installing");
                Status::internal(format!("state transition failed: {}", e))
            })?;

        self.emit_event(
            &adapter_id,
            &AdapterState::Unknown,
            &AdapterState::Installing,
        );

        // Create and start container
        let env_vars = self.default_config.to_env_vars();
        drop(store); // Release lock before async podman operations

        match self
            .runtime
            .create_and_start(&container_name, &image_ref, &env_vars, CONTAINER_NETWORK)
            .await
        {
            Ok(()) => {
                // Installing → Running
                let mut store = self.store.lock().await;
                store
                    .transition(&adapter_id, AdapterState::Running)
                    .map_err(|e| {
                        error!(error = %e, "failed to transition to Running");
                        Status::internal(format!("state transition failed: {}", e))
                    })?;

                self.emit_event(
                    &adapter_id,
                    &AdapterState::Installing,
                    &AdapterState::Running,
                );

                info!(adapter_id = %adapter_id, "adapter installed and running");

                Ok(Response::new(InstallAdapterResponse {
                    job_id,
                    adapter_id,
                    state: AdapterState::Running.to_proto(),
                }))
            }
            Err(e) => {
                // Installing → Error (04-REQ-3.E1, 04-REQ-3.E3)
                let error_msg = e.to_string();
                warn!(
                    adapter_id = %adapter_id,
                    error = %error_msg,
                    "podman create/start failed"
                );

                let mut store = self.store.lock().await;
                let error_state = AdapterState::Error(error_msg.clone());
                let _ = store.transition(&adapter_id, error_state.clone());

                self.emit_event(
                    &adapter_id,
                    &AdapterState::Installing,
                    &error_state,
                );

                Ok(Response::new(InstallAdapterResponse {
                    job_id,
                    adapter_id,
                    state: AdapterState::Error(error_msg).to_proto(),
                }))
            }
        }
    }

    type WatchAdapterStatesStream =
        tokio_stream::wrappers::ReceiverStream<Result<AdapterStateEvent, Status>>;

    /// Stream adapter state events.
    ///
    /// # Requirements
    ///
    /// - 04-REQ-4.3: Server-streaming RPC emitting AdapterStateEvent.
    async fn watch_adapter_states(
        &self,
        _request: Request<WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        info!("WatchAdapterStates stream started");

        let mut rx = self.notifier.subscribe();
        let (tx, out_rx) = tokio::sync::mpsc::channel(64);

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if tx.send(Ok(event)).await.is_err() {
                            // Client disconnected
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(missed = n, "WatchAdapterStates: subscriber lagged");
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            out_rx,
        )))
    }

    /// List all known adapters.
    ///
    /// # Requirements
    ///
    /// - 04-REQ-4.4: Return all adapters with AdapterInfo.
    async fn list_adapters(
        &self,
        _request: Request<ListAdaptersRequest>,
    ) -> Result<Response<ListAdaptersResponse>, Status> {
        let store = self.store.lock().await;
        let adapters = store.list().iter().map(|e| e.to_proto_info()).collect();

        Ok(Response::new(ListAdaptersResponse { adapters }))
    }

    /// Remove an adapter: stop and remove the container.
    ///
    /// # Requirements
    ///
    /// - 04-REQ-3.3: Stop and remove container.
    /// - 04-REQ-4.E2: Unknown adapter_id → NOT_FOUND.
    /// - 04-REQ-5.E1: Cancel offload timer on manual removal.
    async fn remove_adapter(
        &self,
        request: Request<RemoveAdapterRequest>,
    ) -> Result<Response<RemoveAdapterResponse>, Status> {
        let adapter_id = request.into_inner().adapter_id;

        info!(adapter_id = %adapter_id, "RemoveAdapter request");

        // Cancel offload timer if running (04-REQ-5.E1)
        self.offload_mgr.cancel_timer(&adapter_id).await;

        let (container_name, old_state) = {
            let store = self.store.lock().await;
            let entry = store
                .get(&adapter_id)
                .ok_or_else(|| Status::not_found(format!("adapter '{}' not found", adapter_id)))?;
            (entry.container_name.clone(), entry.state.clone())
        };

        // Stop and remove container
        if let Err(e) = self.runtime.stop_and_remove(&container_name).await {
            warn!(
                adapter_id = %adapter_id,
                error = %e,
                "failed to stop/remove container (may already be removed)"
            );
        }

        // Transition to Stopped
        let mut store = self.store.lock().await;
        match store.transition(&adapter_id, AdapterState::Stopped) {
            Ok(_old) => {
                self.emit_event(&adapter_id, &old_state, &AdapterState::Stopped);
            }
            Err(e) => {
                warn!(
                    adapter_id = %adapter_id,
                    error = %e,
                    "transition to Stopped failed (removing entry)"
                );
                // Remove the entry entirely if we can't transition
                let _ = store.remove(&adapter_id);
            }
        }

        info!(adapter_id = %adapter_id, "adapter removed");

        Ok(Response::new(RemoveAdapterResponse {}))
    }

    /// Get the status of a specific adapter.
    ///
    /// # Requirements
    ///
    /// - 04-REQ-4.5: Return AdapterInfo and state.
    /// - 04-REQ-4.E1: Unknown adapter_id → NOT_FOUND.
    async fn get_adapter_status(
        &self,
        request: Request<GetAdapterStatusRequest>,
    ) -> Result<Response<GetAdapterStatusResponse>, Status> {
        let adapter_id = request.into_inner().adapter_id;

        let store = self.store.lock().await;
        let entry = store
            .get(&adapter_id)
            .ok_or_else(|| Status::not_found(format!("adapter '{}' not found", adapter_id)))?;

        Ok(Response::new(GetAdapterStatusResponse {
            info: Some(entry.to_proto_info()),
            state: entry.state.to_proto(),
        }))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::podman::tests::MockContainerRuntime;
    use parking_proto::services::update::update_service_client::UpdateServiceClient;
    use std::net::SocketAddr;
    use std::time::Duration;
    use tokio::net::TcpListener;

    /// Helper: create a test UpdateServiceImpl backed by MockContainerRuntime.
    fn make_test_service(
        store: AdapterStore,
        mock: MockContainerRuntime,
    ) -> UpdateServiceImpl<MockContainerRuntime> {
        let config = AdapterConfig {
            databroker_addr: Some("localhost:55555".to_string()),
            parking_operator_url: Some("http://operator:8082".to_string()),
            zone_id: Some("zone-1".to_string()),
            vehicle_vin: Some("VIN001".to_string()),
            listen_addr: Some("0.0.0.0:50054".to_string()),
        };

        UpdateServiceImpl::new(
            Arc::new(Mutex::new(store)),
            Arc::new(mock),
            OffloadManager::new(Duration::from_secs(300)),
            config,
        )
    }

    /// Helper: start a gRPC server on a random port with the given service.
    async fn start_test_server(
        service: UpdateServiceImpl<MockContainerRuntime>,
    ) -> SocketAddr {
        use parking_proto::services::update::update_service_server::UpdateServiceServer;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(UpdateServiceServer::new(service))
                .serve_with_incoming(incoming)
                .await
                .unwrap();
        });

        addr
    }

    /// Helper: create an empty AdapterStore in a temp directory.
    fn temp_store() -> (tempfile::TempDir, AdapterStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = AdapterStore::load(dir.path().to_str().unwrap()).unwrap();
        (dir, store)
    }

    // ---- StateNotifier tests ----

    #[tokio::test]
    async fn notifier_broadcasts_event() {
        let notifier = StateNotifier::new(16);
        let mut rx = notifier.subscribe();

        let event = AdapterStateEvent {
            adapter_id: "a1".to_string(),
            old_state: 0,
            new_state: 3,
            timestamp: 1000,
        };
        notifier.notify(event.clone());

        let received = rx.recv().await.unwrap();
        assert_eq!(received.adapter_id, "a1");
        assert_eq!(received.new_state, 3);
    }

    #[tokio::test]
    async fn notifier_no_subscriber_does_not_panic() {
        let notifier = StateNotifier::new(16);
        // Should not panic even without subscribers
        notifier.notify(AdapterStateEvent {
            adapter_id: "a1".to_string(),
            old_state: 0,
            new_state: 3,
            timestamp: 1000,
        });
    }

    // ---- InstallAdapter tests ----

    #[tokio::test]
    async fn install_adapter_creates_and_starts_container() {
        let (_dir, store) = temp_store();
        let mock = MockContainerRuntime::new();
        let service = make_test_service(store, mock.clone());

        let addr = start_test_server(service).await;
        let mut client = UpdateServiceClient::connect(format!("http://{}", addr))
            .await
            .unwrap();

        let resp = client
            .install_adapter(InstallAdapterRequest {
                image_ref: "localhost/test:latest".into(),
                checksum: "sha256:abc123".into(),
            })
            .await
            .unwrap()
            .into_inner();

        assert!(!resp.adapter_id.is_empty());
        assert!(!resp.job_id.is_empty());
        assert_eq!(
            resp.state,
            parking_proto::common::AdapterState::Running as i32
        );

        // Verify podman was called
        let calls = mock.get_calls();
        assert!(calls.iter().any(|c| matches!(
            c,
            crate::podman::tests::PodmanCall::CreateAndStart { .. }
        )));
    }

    #[tokio::test]
    async fn install_adapter_passes_env_vars() {
        let (_dir, store) = temp_store();
        let mock = MockContainerRuntime::new();
        let service = make_test_service(store, mock.clone());

        let addr = start_test_server(service).await;
        let mut client = UpdateServiceClient::connect(format!("http://{}", addr))
            .await
            .unwrap();

        client
            .install_adapter(InstallAdapterRequest {
                image_ref: "localhost/test:latest".into(),
                checksum: "sha256:abc".into(),
            })
            .await
            .unwrap();

        // Verify env vars
        let calls = mock.get_calls();
        let create_call = calls
            .iter()
            .find(|c| matches!(c, crate::podman::tests::PodmanCall::CreateAndStart { .. }))
            .unwrap();

        if let crate::podman::tests::PodmanCall::CreateAndStart {
            env_vars, network, ..
        } = create_call
        {
            assert_eq!(env_vars.get("DATABROKER_ADDR").unwrap(), "localhost:55555");
            assert_eq!(
                env_vars.get("PARKING_OPERATOR_URL").unwrap(),
                "http://operator:8082"
            );
            assert_eq!(env_vars.get("ZONE_ID").unwrap(), "zone-1");
            assert_eq!(env_vars.get("VEHICLE_VIN").unwrap(), "VIN001");
            assert_eq!(env_vars.get("LISTEN_ADDR").unwrap(), "0.0.0.0:50054");
            assert_eq!(network, "host");
        }
    }

    #[tokio::test]
    async fn install_adapter_already_running_returns_existing() {
        let (_dir, mut store) = temp_store();

        // Pre-insert a running adapter
        let entry = AdapterEntry {
            adapter_id: "existing-1".to_string(),
            image_ref: "localhost/test:latest".to_string(),
            checksum: "sha256:abc".to_string(),
            container_name: "poa-existing-1".to_string(),
            state: AdapterState::Running,
            config: AdapterConfig::default(),
            installed_at: Some(1000),
            session_ended_at: None,
        };
        store.insert(entry).unwrap();

        let mock = MockContainerRuntime::new().with_running("poa-existing-1");
        let service = make_test_service(store, mock.clone());

        let addr = start_test_server(service).await;
        let mut client = UpdateServiceClient::connect(format!("http://{}", addr))
            .await
            .unwrap();

        let resp = client
            .install_adapter(InstallAdapterRequest {
                image_ref: "localhost/test:latest".into(),
                checksum: "sha256:abc".into(),
            })
            .await
            .unwrap()
            .into_inner();

        // Should return the existing adapter
        assert_eq!(resp.adapter_id, "existing-1");
        assert_eq!(
            resp.state,
            parking_proto::common::AdapterState::Running as i32
        );

        // No new container should have been created
        let calls = mock.get_calls();
        assert!(!calls.iter().any(|c| matches!(
            c,
            crate::podman::tests::PodmanCall::CreateAndStart { .. }
        )));
    }

    #[tokio::test]
    async fn install_adapter_podman_failure_sets_error_state() {
        let (_dir, store) = temp_store();
        let mock = MockContainerRuntime::new().with_fail_create();
        let service = make_test_service(store, mock);

        let addr = start_test_server(service).await;
        let mut client = UpdateServiceClient::connect(format!("http://{}", addr))
            .await
            .unwrap();

        let resp = client
            .install_adapter(InstallAdapterRequest {
                image_ref: "localhost/nonexistent:latest".into(),
                checksum: "sha256:abc".into(),
            })
            .await
            .unwrap()
            .into_inner();

        // State should be ERROR
        assert_eq!(
            resp.state,
            parking_proto::common::AdapterState::Error as i32
        );
    }

    // ---- ListAdapters tests ----

    #[tokio::test]
    async fn list_adapters_empty() {
        let (_dir, store) = temp_store();
        let mock = MockContainerRuntime::new();
        let service = make_test_service(store, mock);

        let addr = start_test_server(service).await;
        let mut client = UpdateServiceClient::connect(format!("http://{}", addr))
            .await
            .unwrap();

        let resp = client
            .list_adapters(ListAdaptersRequest {})
            .await
            .unwrap()
            .into_inner();

        assert!(resp.adapters.is_empty());
    }

    #[tokio::test]
    async fn list_adapters_with_entries() {
        let (_dir, store) = temp_store();
        let mock = MockContainerRuntime::new();
        let service = make_test_service(store, mock);

        let addr = start_test_server(service).await;
        let mut client = UpdateServiceClient::connect(format!("http://{}", addr))
            .await
            .unwrap();

        // Install two adapters
        client
            .install_adapter(InstallAdapterRequest {
                image_ref: "img-1:latest".into(),
                checksum: "sha256:a".into(),
            })
            .await
            .unwrap();

        client
            .install_adapter(InstallAdapterRequest {
                image_ref: "img-2:latest".into(),
                checksum: "sha256:b".into(),
            })
            .await
            .unwrap();

        let resp = client
            .list_adapters(ListAdaptersRequest {})
            .await
            .unwrap()
            .into_inner();

        assert_eq!(resp.adapters.len(), 2);
    }

    // ---- GetAdapterStatus tests ----

    #[tokio::test]
    async fn get_adapter_status_found() {
        let (_dir, store) = temp_store();
        let mock = MockContainerRuntime::new();
        let service = make_test_service(store, mock);

        let addr = start_test_server(service).await;
        let mut client = UpdateServiceClient::connect(format!("http://{}", addr))
            .await
            .unwrap();

        // Install an adapter
        let install_resp = client
            .install_adapter(InstallAdapterRequest {
                image_ref: "test:latest".into(),
                checksum: "sha256:a".into(),
            })
            .await
            .unwrap()
            .into_inner();

        // Get its status
        let resp = client
            .get_adapter_status(GetAdapterStatusRequest {
                adapter_id: install_resp.adapter_id.clone(),
            })
            .await
            .unwrap()
            .into_inner();

        assert!(resp.info.is_some());
        assert_eq!(resp.info.unwrap().adapter_id, install_resp.adapter_id);
        assert_eq!(
            resp.state,
            parking_proto::common::AdapterState::Running as i32
        );
    }

    #[tokio::test]
    async fn get_adapter_status_not_found() {
        let (_dir, store) = temp_store();
        let mock = MockContainerRuntime::new();
        let service = make_test_service(store, mock);

        let addr = start_test_server(service).await;
        let mut client = UpdateServiceClient::connect(format!("http://{}", addr))
            .await
            .unwrap();

        let status = client
            .get_adapter_status(GetAdapterStatusRequest {
                adapter_id: "nonexistent".into(),
            })
            .await
            .unwrap_err();

        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    // ---- RemoveAdapter tests ----

    #[tokio::test]
    async fn remove_adapter_stops_and_removes() {
        let (_dir, store) = temp_store();
        let mock = MockContainerRuntime::new();
        let service = make_test_service(store, mock.clone());

        let addr = start_test_server(service).await;
        let mut client = UpdateServiceClient::connect(format!("http://{}", addr))
            .await
            .unwrap();

        // Install
        let install_resp = client
            .install_adapter(InstallAdapterRequest {
                image_ref: "test:latest".into(),
                checksum: "sha256:a".into(),
            })
            .await
            .unwrap()
            .into_inner();

        // Remove
        client
            .remove_adapter(RemoveAdapterRequest {
                adapter_id: install_resp.adapter_id.clone(),
            })
            .await
            .unwrap();

        // Verify podman stop+remove was called
        let calls = mock.get_calls();
        assert!(calls.iter().any(|c| matches!(
            c,
            crate::podman::tests::PodmanCall::StopAndRemove { .. }
        )));

        // Status should reflect Stopped
        let status = client
            .get_adapter_status(GetAdapterStatusRequest {
                adapter_id: install_resp.adapter_id.clone(),
            })
            .await
            .unwrap()
            .into_inner();

        assert_eq!(
            status.state,
            parking_proto::common::AdapterState::Stopped as i32
        );
    }

    #[tokio::test]
    async fn remove_adapter_not_found() {
        let (_dir, store) = temp_store();
        let mock = MockContainerRuntime::new();
        let service = make_test_service(store, mock);

        let addr = start_test_server(service).await;
        let mut client = UpdateServiceClient::connect(format!("http://{}", addr))
            .await
            .unwrap();

        let status = client
            .remove_adapter(RemoveAdapterRequest {
                adapter_id: "nonexistent".into(),
            })
            .await
            .unwrap_err();

        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    // ---- WatchAdapterStates tests ----

    #[tokio::test]
    async fn watch_adapter_states_receives_events() {
        let (_dir, store) = temp_store();
        let mock = MockContainerRuntime::new();
        let service = make_test_service(store, mock);

        let addr = start_test_server(service).await;
        let mut client = UpdateServiceClient::connect(format!("http://{}", addr))
            .await
            .unwrap();

        // Start watching
        let mut stream = client
            .watch_adapter_states(WatchAdapterStatesRequest {})
            .await
            .unwrap()
            .into_inner();

        // Install an adapter (triggers events)
        let install_resp = client
            .install_adapter(InstallAdapterRequest {
                image_ref: "test:latest".into(),
                checksum: "sha256:a".into(),
            })
            .await
            .unwrap()
            .into_inner();

        // Should receive Unknown → Installing and Installing → Running events
        let event1 = tokio::time::timeout(Duration::from_secs(2), stream.message())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        assert_eq!(event1.adapter_id, install_resp.adapter_id);
        assert_eq!(
            event1.old_state,
            parking_proto::common::AdapterState::Unknown as i32
        );
        assert_eq!(
            event1.new_state,
            parking_proto::common::AdapterState::Installing as i32
        );

        let event2 = tokio::time::timeout(Duration::from_secs(2), stream.message())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        assert_eq!(event2.adapter_id, install_resp.adapter_id);
        assert_eq!(
            event2.old_state,
            parking_proto::common::AdapterState::Installing as i32
        );
        assert_eq!(
            event2.new_state,
            parking_proto::common::AdapterState::Running as i32
        );
    }

    // ---- Reconciliation tests ----

    #[tokio::test]
    async fn reconcile_running_container_stays_running() {
        let (_dir, mut store) = temp_store();

        // Pre-insert a running adapter
        let entry = AdapterEntry {
            adapter_id: "a1".to_string(),
            image_ref: "test:latest".to_string(),
            checksum: "sha256:abc".to_string(),
            container_name: "poa-a1".to_string(),
            state: AdapterState::Running,
            config: AdapterConfig::default(),
            installed_at: Some(1000),
            session_ended_at: None,
        };
        store.insert(entry).unwrap();

        let mock = MockContainerRuntime::new().with_running("poa-a1");
        let service = make_test_service(store, mock);

        service.reconcile().await;

        let store = service.store.lock().await;
        assert_eq!(store.get("a1").unwrap().state, AdapterState::Running);
    }

    #[tokio::test]
    async fn reconcile_dead_container_transitions_to_error() {
        let (_dir, mut store) = temp_store();

        // Pre-insert a running adapter whose container is NOT actually running
        let entry = AdapterEntry {
            adapter_id: "a1".to_string(),
            image_ref: "test:latest".to_string(),
            checksum: "sha256:abc".to_string(),
            container_name: "poa-a1".to_string(),
            state: AdapterState::Running,
            config: AdapterConfig::default(),
            installed_at: Some(1000),
            session_ended_at: None,
        };
        store.insert(entry).unwrap();

        let mock = MockContainerRuntime::new(); // No running containers
        let service = make_test_service(store, mock);

        service.reconcile().await;

        let store = service.store.lock().await;
        assert!(matches!(
            store.get("a1").unwrap().state,
            AdapterState::Error(_)
        ));
    }
}
