//! gRPC service implementation for the UPDATE_SERVICE.
//!
//! Wraps [`UpdateServiceImpl`] and exposes the five RPCs defined in
//! `update_service.proto` via the tonic-generated [`UpdateService`] trait.

use crate::adapter::{self, AdapterState};
use crate::podman::{PodmanExecutor, UpdateServiceImpl};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;
use tonic::{Request, Response, Status};

// ── Generated proto code ──────────────────────────────────────────────────────

#[allow(clippy::enum_variant_names)]
pub mod proto {
    tonic::include_proto!("update");
}

use proto::{
    update_service_server::UpdateService, AdapterInfo, AdapterState as ProtoAdapterState,
    AdapterStateEvent as ProtoAdapterStateEvent, GetAdapterStatusRequest,
    GetAdapterStatusResponse, InstallAdapterRequest, InstallAdapterResponse, ListAdaptersRequest,
    ListAdaptersResponse, RemoveAdapterRequest, RemoveAdapterResponse, WatchAdapterStatesRequest,
};

// ── Type conversions ──────────────────────────────────────────────────────────

/// Convert an internal `AdapterState` to the proto enum integer value.
///
/// prost strips the common `ADAPTER_STATE_` prefix from the proto enum variant
/// names, so `ADAPTER_STATE_UNKNOWN` becomes `ProtoAdapterState::Unknown`, etc.
fn state_to_proto(state: &AdapterState) -> i32 {
    match state {
        AdapterState::Unknown => ProtoAdapterState::Unknown as i32,
        AdapterState::Downloading => ProtoAdapterState::Downloading as i32,
        AdapterState::Installing => ProtoAdapterState::Installing as i32,
        AdapterState::Running => ProtoAdapterState::Running as i32,
        AdapterState::Stopped => ProtoAdapterState::Stopped as i32,
        AdapterState::Error => ProtoAdapterState::Error as i32,
        AdapterState::Offloading => ProtoAdapterState::Offloading as i32,
    }
}

/// Convert an internal `AdapterStateEvent` to the proto message.
fn event_to_proto(event: adapter::AdapterStateEvent) -> ProtoAdapterStateEvent {
    ProtoAdapterStateEvent {
        adapter_id: event.adapter_id,
        old_state: state_to_proto(&event.old_state),
        new_state: state_to_proto(&event.new_state),
        // timestamp is stored as Unix milliseconds (07-REQ-8.2 / proto comment).
        timestamp: event.timestamp as i64,
    }
}

/// Convert an internal `AdapterEntry` to the proto `AdapterInfo` message.
fn adapter_entry_to_info(entry: &adapter::AdapterEntry) -> AdapterInfo {
    AdapterInfo {
        adapter_id: entry.adapter_id.clone(),
        image_ref: entry.image_ref.clone(),
        state: state_to_proto(&entry.state),
    }
}

// ── gRPC service struct ───────────────────────────────────────────────────────

/// The tonic gRPC service that implements all five UPDATE_SERVICE RPCs.
pub struct GrpcUpdateService<P: PodmanExecutor + Send + Sync + 'static> {
    inner: Arc<UpdateServiceImpl<P>>,
    broadcaster: broadcast::Sender<adapter::AdapterStateEvent>,
}

impl<P: PodmanExecutor + Send + Sync + 'static> GrpcUpdateService<P> {
    pub fn new(
        inner: Arc<UpdateServiceImpl<P>>,
        broadcaster: broadcast::Sender<adapter::AdapterStateEvent>,
    ) -> Self {
        Self { inner, broadcaster }
    }
}

// ── tonic trait implementation ────────────────────────────────────────────────

#[tonic::async_trait]
impl<P: PodmanExecutor + Send + Sync + 'static> UpdateService for GrpcUpdateService<P> {
    type WatchAdapterStatesStream =
        Pin<Box<dyn Stream<Item = Result<ProtoAdapterStateEvent, Status>> + Send>>;

    /// InstallAdapter: validate inputs, derive adapter_id, start background
    /// install, and return immediately with DOWNLOADING state (07-REQ-1.1).
    async fn install_adapter(
        &self,
        request: Request<InstallAdapterRequest>,
    ) -> Result<Response<InstallAdapterResponse>, Status> {
        let req = request.into_inner();
        let resp = self
            .inner
            .install_adapter(&req.image_ref, &req.checksum_sha256)
            .await?;
        Ok(Response::new(InstallAdapterResponse {
            job_id: resp.job_id,
            adapter_id: resp.adapter_id,
            state: state_to_proto(&resp.state),
        }))
    }

    /// WatchAdapterStates: subscribe to the broadcast channel and stream all
    /// future state-change events to the caller (07-REQ-3.1 through 07-REQ-3.4).
    ///
    /// No historical events are replayed — only transitions that occur after
    /// the subscription is established are delivered.
    ///
    /// Lagged events (when the subscriber falls behind the broadcast buffer) are
    /// silently skipped; the stream does not terminate on lag.
    async fn watch_adapter_states(
        &self,
        _request: Request<WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        let rx = self.broadcaster.subscribe();

        let stream = futures::stream::unfold(rx, |mut rx| async move {
            loop {
                match rx.recv().await {
                    Ok(event) => return Some((Ok(event_to_proto(event)), rx)),
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // Subscriber fell behind; skip the missed events and continue.
                        tracing::warn!(
                            "WatchAdapterStates subscriber lagged by {n} messages; skipping"
                        );
                        // Loop to try the next recv.
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        // Channel closed (service shutting down); end the stream.
                        return None;
                    }
                }
            }
        });

        Ok(Response::new(Box::pin(stream)))
    }

    /// ListAdapters: return all currently known adapters (07-REQ-4.1).
    async fn list_adapters(
        &self,
        _request: Request<ListAdaptersRequest>,
    ) -> Result<Response<ListAdaptersResponse>, Status> {
        let adapters = self.inner.list_adapters();
        let adapter_infos = adapters.iter().map(adapter_entry_to_info).collect();
        Ok(Response::new(ListAdaptersResponse {
            adapters: adapter_infos,
        }))
    }

    /// RemoveAdapter: stop the container (if running), remove it and its image,
    /// and purge from state (07-REQ-5.1, 07-REQ-5.2).
    async fn remove_adapter(
        &self,
        request: Request<RemoveAdapterRequest>,
    ) -> Result<Response<RemoveAdapterResponse>, Status> {
        let req = request.into_inner();
        self.inner.remove_adapter(&req.adapter_id).await?;
        Ok(Response::new(RemoveAdapterResponse {}))
    }

    /// GetAdapterStatus: return the current state of a specific adapter, or
    /// NOT_FOUND if the adapter_id is unknown (07-REQ-4.2, 07-REQ-4.E1).
    async fn get_adapter_status(
        &self,
        request: Request<GetAdapterStatusRequest>,
    ) -> Result<Response<GetAdapterStatusResponse>, Status> {
        let req = request.into_inner();
        let entry = self.inner.get_adapter_status(&req.adapter_id)?;
        Ok(Response::new(GetAdapterStatusResponse {
            adapter: Some(adapter_entry_to_info(&entry)),
        }))
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::podman::{MockPodmanExecutor, PodmanError};
    use crate::state::StateManager;
    use tokio::sync::broadcast;

    const IMAGE_REF: &str = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
    const CHECKSUM: &str = "sha256:abc123";
    const ADAPTER_ID: &str = "parkhaus-munich-v1.0.0";

    fn make_grpc_service(
        mock: Arc<MockPodmanExecutor>,
    ) -> (
        GrpcUpdateService<MockPodmanExecutor>,
        Arc<StateManager>,
        broadcast::Receiver<adapter::AdapterStateEvent>,
    ) {
        let (tx, rx) = broadcast::channel(64);
        let state = Arc::new(StateManager::new(tx.clone()));
        let inner = Arc::new(UpdateServiceImpl::new(
            Arc::clone(&state),
            Arc::clone(&mock),
            tx.clone(),
        ));
        let svc = GrpcUpdateService::new(inner, tx);
        (svc, state, rx)
    }

    /// TS-07-E1: empty image_ref returns INVALID_ARGUMENT via gRPC layer.
    #[tokio::test]
    async fn test_grpc_install_empty_image_ref() {
        let mock = Arc::new(MockPodmanExecutor::new());
        let (svc, _state, _rx) = make_grpc_service(mock);
        let result = svc
            .install_adapter(Request::new(InstallAdapterRequest {
                image_ref: String::new(),
                checksum_sha256: CHECKSUM.to_string(),
            }))
            .await;
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert!(status.message().contains("image_ref is required"));
    }

    /// TS-07-E2: empty checksum_sha256 returns INVALID_ARGUMENT via gRPC layer.
    #[tokio::test]
    async fn test_grpc_install_empty_checksum() {
        let mock = Arc::new(MockPodmanExecutor::new());
        let (svc, _state, _rx) = make_grpc_service(mock);
        let result = svc
            .install_adapter(Request::new(InstallAdapterRequest {
                image_ref: IMAGE_REF.to_string(),
                checksum_sha256: String::new(),
            }))
            .await;
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert!(status.message().contains("checksum_sha256 is required"));
    }

    /// TS-07-E8: GetAdapterStatus unknown ID returns NOT_FOUND via gRPC layer.
    #[tokio::test]
    async fn test_grpc_get_unknown_adapter() {
        let mock = Arc::new(MockPodmanExecutor::new());
        let (svc, _state, _rx) = make_grpc_service(mock);
        let result = svc
            .get_adapter_status(Request::new(GetAdapterStatusRequest {
                adapter_id: "nonexistent-adapter".to_string(),
            }))
            .await;
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
        assert!(status.message().contains("adapter not found"));
    }

    /// TS-07-E9: ListAdapters returns empty response when no adapters installed.
    #[tokio::test]
    async fn test_grpc_list_adapters_empty() {
        let mock = Arc::new(MockPodmanExecutor::new());
        let (svc, _state, _rx) = make_grpc_service(mock);
        let resp = svc
            .list_adapters(Request::new(ListAdaptersRequest {}))
            .await
            .unwrap()
            .into_inner();
        assert!(resp.adapters.is_empty());
    }

    /// TS-07-E10: RemoveAdapter unknown ID returns NOT_FOUND via gRPC layer.
    #[tokio::test]
    async fn test_grpc_remove_unknown_adapter() {
        let mock = Arc::new(MockPodmanExecutor::new());
        let (svc, _state, _rx) = make_grpc_service(mock);
        let result = svc
            .remove_adapter(Request::new(RemoveAdapterRequest {
                adapter_id: "nonexistent-adapter".to_string(),
            }))
            .await;
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
        assert!(status.message().contains("adapter not found"));
    }

    /// TS-07-E11: RemoveAdapter with podman rm failure returns INTERNAL via gRPC layer.
    #[tokio::test]
    async fn test_grpc_removal_failure_internal() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_rm_result(Err(PodmanError::new("container in use")));
        let (svc, state, _rx) = make_grpc_service(Arc::clone(&mock));

        // Place adapter directly in RUNNING state to avoid background task races.
        state.create_adapter(crate::adapter::AdapterEntry {
            adapter_id: ADAPTER_ID.to_string(),
            image_ref: IMAGE_REF.to_string(),
            checksum_sha256: CHECKSUM.to_string(),
            state: crate::adapter::AdapterState::Running,
            job_id: "job-direct".to_string(),
            stopped_at: None,
            error_message: None,
        });

        let result = svc
            .remove_adapter(Request::new(RemoveAdapterRequest {
                adapter_id: ADAPTER_ID.to_string(),
            }))
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::Internal);
    }

    /// TS-07-1: install_adapter returns job_id, adapter_id, DOWNLOADING via gRPC layer.
    #[tokio::test]
    async fn test_grpc_install_response_immediate() {
        let mock = Arc::new(MockPodmanExecutor::new());
        // wait not set → blocks, adapter stays RUNNING
        let (svc, _state, _rx) = make_grpc_service(mock);

        let resp = svc
            .install_adapter(Request::new(InstallAdapterRequest {
                image_ref: IMAGE_REF.to_string(),
                checksum_sha256: CHECKSUM.to_string(),
            }))
            .await
            .unwrap()
            .into_inner();

        assert!(!resp.job_id.is_empty());
        assert_eq!(resp.adapter_id, ADAPTER_ID);
        assert_eq!(resp.state, ProtoAdapterState::Downloading as i32);
    }

    /// TS-07-11: GetAdapterStatus returns current state via gRPC layer.
    #[tokio::test]
    async fn test_grpc_get_adapter_status() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        // wait not set → blocks, adapter stays RUNNING
        let (svc, _state, _rx) = make_grpc_service(mock);

        svc.install_adapter(Request::new(InstallAdapterRequest {
            image_ref: IMAGE_REF.to_string(),
            checksum_sha256: CHECKSUM.to_string(),
        }))
        .await
        .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let resp = svc
            .get_adapter_status(Request::new(GetAdapterStatusRequest {
                adapter_id: ADAPTER_ID.to_string(),
            }))
            .await
            .unwrap()
            .into_inner();

        let info = resp.adapter.expect("adapter field must be present");
        assert_eq!(info.adapter_id, ADAPTER_ID);
        assert_eq!(info.state, ProtoAdapterState::Running as i32);
    }

    /// TS-07-8: WatchAdapterStates streams events with correct fields.
    #[tokio::test]
    async fn test_grpc_watch_adapter_states() {
        use futures::StreamExt;

        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM.to_string()));
        mock.set_run_result(Ok(()));
        // wait not set → blocks, adapter stays RUNNING
        let (svc, _state, _rx) = make_grpc_service(mock);

        // Subscribe before install.
        let resp = svc
            .watch_adapter_states(Request::new(WatchAdapterStatesRequest {}))
            .await
            .unwrap();
        let mut stream = resp.into_inner();

        // Trigger install in a background task so we can collect events.
        let _ = svc
            .install_adapter(Request::new(InstallAdapterRequest {
                image_ref: IMAGE_REF.to_string(),
                checksum_sha256: CHECKSUM.to_string(),
            }))
            .await
            .unwrap();

        // Collect events with a short timeout.
        let mut events = Vec::new();
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(500);
        loop {
            match tokio::time::timeout_at(deadline, stream.next()).await {
                Ok(Some(Ok(event))) => events.push(event),
                _ => break,
            }
        }

        // Should have at least UNKNOWN->DOWNLOADING (emitted synchronously).
        assert!(
            !events.is_empty(),
            "expected at least one state event, got none"
        );
        // First event must be UNKNOWN -> DOWNLOADING.
        assert_eq!(events[0].old_state, ProtoAdapterState::Unknown as i32);
        assert_eq!(events[0].new_state, ProtoAdapterState::Downloading as i32);
        assert_eq!(events[0].adapter_id, ADAPTER_ID);
        assert!(events[0].timestamp > 0);
    }
}
