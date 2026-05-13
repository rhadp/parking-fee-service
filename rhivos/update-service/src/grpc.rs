use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};

use crate::adapter::{AdapterState as InternalState, AdapterStateEvent as InternalEvent};
use crate::install;
use crate::podman::PodmanExecutor;
use crate::state::StateManager;

/// Generated protobuf types.
pub mod proto {
    tonic::include_proto!("update_service.v1");
}

/// Converts an internal `AdapterState` to the protobuf enum integer value.
fn state_to_proto(state: &InternalState) -> i32 {
    match state {
        InternalState::Unknown => proto::AdapterState::Unknown as i32,
        InternalState::Downloading => proto::AdapterState::Downloading as i32,
        InternalState::Installing => proto::AdapterState::Installing as i32,
        InternalState::Running => proto::AdapterState::Running as i32,
        InternalState::Stopped => proto::AdapterState::Stopped as i32,
        InternalState::Error => proto::AdapterState::Error as i32,
        InternalState::Offloading => proto::AdapterState::Offloading as i32,
    }
}

/// The gRPC service implementation for UpdateService.
pub struct UpdateServiceImpl {
    state_mgr: Arc<StateManager>,
    podman: Arc<dyn PodmanExecutor>,
    broadcast_tx: broadcast::Sender<InternalEvent>,
}

impl UpdateServiceImpl {
    pub fn new(
        state_mgr: Arc<StateManager>,
        podman: Arc<dyn PodmanExecutor>,
        broadcast_tx: broadcast::Sender<InternalEvent>,
    ) -> Self {
        Self {
            state_mgr,
            podman,
            broadcast_tx,
        }
    }
}

type WatchStream = Pin<
    Box<dyn tokio_stream::Stream<Item = Result<proto::AdapterStateEvent, Status>> + Send>,
>;

#[tonic::async_trait]
impl proto::update_service_server::UpdateService for UpdateServiceImpl {
    /// InstallAdapter validates inputs, delegates to the install orchestration,
    /// and returns the response immediately.
    async fn install_adapter(
        &self,
        request: Request<proto::InstallAdapterRequest>,
    ) -> Result<Response<proto::InstallAdapterResponse>, Status> {
        let req = request.into_inner();

        match install::install_adapter(
            &req.image_ref,
            &req.checksum_sha256,
            self.state_mgr.clone(),
            self.podman.clone(),
        )
        .await
        {
            Ok((job_id, adapter_id, initial_state)) => {
                let resp = proto::InstallAdapterResponse {
                    job_id,
                    adapter_id,
                    state: state_to_proto(&initial_state),
                };
                Ok(Response::new(resp))
            }
            Err(install::InstallError::InvalidArgument(msg)) => {
                Err(Status::invalid_argument(msg))
            }
        }
    }

    type WatchAdapterStatesStream = WatchStream;

    /// WatchAdapterStates subscribes to the broadcast channel and streams
    /// state events to the client. Only new events after subscription are
    /// delivered (no historical replay).
    async fn watch_adapter_states(
        &self,
        _request: Request<proto::WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        let rx = self.broadcast_tx.subscribe();
        let stream = BroadcastStream::new(rx).filter_map(|result| match result {
            Ok(event) => Some(Ok(proto::AdapterStateEvent {
                adapter_id: event.adapter_id,
                old_state: state_to_proto(&event.old_state),
                new_state: state_to_proto(&event.new_state),
                timestamp: event.timestamp as i64,
            })),
            // Lagged receivers get a RecvError::Lagged — skip silently.
            Err(_) => None,
        });
        Ok(Response::new(Box::pin(stream) as Self::WatchAdapterStatesStream))
    }

    /// ListAdapters returns all known adapters with their current states.
    async fn list_adapters(
        &self,
        _request: Request<proto::ListAdaptersRequest>,
    ) -> Result<Response<proto::ListAdaptersResponse>, Status> {
        let adapters = self.state_mgr.list_adapters();
        let proto_adapters = adapters
            .into_iter()
            .map(|a| proto::AdapterInfo {
                adapter_id: a.adapter_id,
                image_ref: a.image_ref,
                state: state_to_proto(&a.state),
            })
            .collect();
        Ok(Response::new(proto::ListAdaptersResponse {
            adapters: proto_adapters,
        }))
    }

    /// RemoveAdapter stops the container (if running), removes container and
    /// image, and removes the adapter from in-memory state.
    async fn remove_adapter(
        &self,
        request: Request<proto::RemoveAdapterRequest>,
    ) -> Result<Response<proto::RemoveAdapterResponse>, Status> {
        let req = request.into_inner();

        // Look up the adapter to get image_ref for the response.
        let adapter = self
            .state_mgr
            .get_adapter(&req.adapter_id)
            .ok_or_else(|| Status::not_found("adapter not found"))?;

        let adapter_id = adapter.adapter_id.clone();

        match install::remove_adapter(&req.adapter_id, self.state_mgr.clone(), self.podman.clone())
            .await
        {
            Ok(()) => Ok(Response::new(proto::RemoveAdapterResponse {
                adapter_id,
                state: proto::AdapterState::Unknown as i32,
            })),
            Err(install::RemoveError::NotFound(msg)) => Err(Status::not_found(msg)),
            Err(install::RemoveError::PodmanFailed(msg)) => Err(Status::internal(msg)),
        }
    }

    /// GetAdapterStatus returns the current state of a specific adapter.
    async fn get_adapter_status(
        &self,
        request: Request<proto::GetAdapterStatusRequest>,
    ) -> Result<Response<proto::GetAdapterStatusResponse>, Status> {
        let req = request.into_inner();
        let entry = self
            .state_mgr
            .get_adapter(&req.adapter_id)
            .ok_or_else(|| Status::not_found("adapter not found"))?;

        Ok(Response::new(proto::GetAdapterStatusResponse {
            adapter: Some(proto::AdapterInfo {
                adapter_id: entry.adapter_id,
                image_ref: entry.image_ref,
                state: state_to_proto(&entry.state),
            }),
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::sync::broadcast;
    use tokio_stream::StreamExt;
    use tonic::{Code, Request};

    use super::proto;
    use super::UpdateServiceImpl;
    use crate::adapter::{
        AdapterEntry, AdapterState as InternalState, AdapterStateEvent as InternalEvent,
    };
    use crate::podman::testing::MockPodmanExecutor;
    use crate::state::StateManager;

    /// TS-07-8: WatchAdapterStates gRPC streaming handler delivers events.
    ///
    /// This test directly invokes the gRPC handler (no network transport) and
    /// asserts that state transitions produce events with correct fields,
    /// covering ≥3 transitions (UNKNOWN→DOWNLOADING, DOWNLOADING→INSTALLING,
    /// INSTALLING→RUNNING) as specified in the test pseudocode.
    ///
    /// Requirements: 07-REQ-3.1, 07-REQ-3.2, 07-REQ-3.3
    #[tokio::test]
    async fn test_grpc_watch_adapter_states_delivers_events() {
        use proto::update_service_server::UpdateService;

        let (tx, _rx) = broadcast::channel::<InternalEvent>(64);
        let state_mgr = Arc::new(StateManager::new(tx.clone()));
        let mock_podman = Arc::new(MockPodmanExecutor::new());
        let service = UpdateServiceImpl::new(state_mgr.clone(), mock_podman, tx);

        // Subscribe via the gRPC handler — receives only future events (no replay).
        let response = service
            .watch_adapter_states(Request::new(proto::WatchAdapterStatesRequest {
                adapter_id: String::new(),
            }))
            .await
            .expect("watch_adapter_states should succeed");
        let mut stream = response.into_inner();

        // Create an adapter and drive it through three state transitions.
        let entry = AdapterEntry {
            adapter_id: "watch-test-v1".to_string(),
            image_ref: "example.com/watch-test:v1".to_string(),
            checksum_sha256: "sha256:abc".to_string(),
            state: InternalState::Unknown,
            job_id: "job-1".to_string(),
            stopped_at: None,
            error_message: None,
        };
        state_mgr.create_adapter(entry);

        // Transition 1: Unknown -> Downloading.
        state_mgr
            .transition("watch-test-v1", InternalState::Downloading, None)
            .expect("transition should succeed");

        let event = tokio::time::timeout(Duration::from_millis(500), stream.next())
            .await
            .expect("timed out waiting for event 1")
            .expect("stream ended")
            .expect("stream error");

        assert_eq!(event.adapter_id, "watch-test-v1", "adapter_id mismatch");
        assert_eq!(
            event.old_state,
            proto::AdapterState::Unknown as i32,
            "event 1 old_state should be Unknown"
        );
        assert_eq!(
            event.new_state,
            proto::AdapterState::Downloading as i32,
            "event 1 new_state should be Downloading"
        );
        assert!(event.timestamp > 0, "timestamp should be a positive Unix epoch");

        // Transition 2: Downloading -> Installing.
        state_mgr
            .transition("watch-test-v1", InternalState::Installing, None)
            .expect("transition should succeed");

        let event2 = tokio::time::timeout(Duration::from_millis(500), stream.next())
            .await
            .expect("timed out waiting for event 2")
            .expect("stream ended")
            .expect("stream error");

        assert_eq!(
            event2.old_state,
            proto::AdapterState::Downloading as i32,
            "event 2 old_state should be Downloading"
        );
        assert_eq!(
            event2.new_state,
            proto::AdapterState::Installing as i32,
            "event 2 new_state should be Installing"
        );

        // Transition 3: Installing -> Running.
        state_mgr
            .transition("watch-test-v1", InternalState::Running, None)
            .expect("transition should succeed");

        let event3 = tokio::time::timeout(Duration::from_millis(500), stream.next())
            .await
            .expect("timed out waiting for event 3")
            .expect("stream ended")
            .expect("stream error");

        assert_eq!(
            event3.old_state,
            proto::AdapterState::Installing as i32,
            "event 3 old_state should be Installing"
        );
        assert_eq!(
            event3.new_state,
            proto::AdapterState::Running as i32,
            "event 3 new_state should be Running"
        );
    }

    /// TS-07-E11: RemoveAdapter gRPC handler returns INTERNAL status when
    /// podman rm fails.
    ///
    /// The install.rs unit test `test_removal_failure_internal` verifies the
    /// orchestration layer returns an error. This test verifies the gRPC
    /// handler maps `RemoveError::PodmanFailed` to `Status::internal`,
    /// satisfying the requirement that the gRPC status code is INTERNAL.
    ///
    /// Requirement: 07-REQ-5.E2
    #[tokio::test]
    async fn test_grpc_removal_failure_returns_internal() {
        use proto::update_service_server::UpdateService;

        let (tx, _rx) = broadcast::channel::<InternalEvent>(64);
        let state_mgr = Arc::new(StateManager::new(tx.clone()));
        let mock_podman = Arc::new(MockPodmanExecutor::new());
        // Configure mock to fail on rm.
        mock_podman.set_rm_result(Err(crate::podman::PodmanError::new("container in use")));

        // Create a STOPPED adapter (no stop call needed during removal).
        let entry = AdapterEntry {
            adapter_id: "fail-rm-v1".to_string(),
            image_ref: "example.com/fail-rm:v1".to_string(),
            checksum_sha256: "sha256:test".to_string(),
            state: InternalState::Stopped,
            job_id: "job-1".to_string(),
            stopped_at: None,
            error_message: None,
        };
        state_mgr.create_adapter(entry);

        let service = UpdateServiceImpl::new(state_mgr.clone(), mock_podman, tx);

        // Call remove_adapter via the gRPC handler.
        let result = service
            .remove_adapter(Request::new(proto::RemoveAdapterRequest {
                adapter_id: "fail-rm-v1".to_string(),
            }))
            .await;

        assert!(result.is_err(), "remove_adapter should fail when podman rm fails");
        let status = result.unwrap_err();
        assert_eq!(
            status.code(),
            Code::Internal,
            "expected gRPC INTERNAL status code, got {:?}",
            status.code()
        );

        // Adapter should be in ERROR state (not removed from state manager).
        let adapter = state_mgr
            .get_adapter("fail-rm-v1")
            .expect("adapter should still be in state manager after failed removal");
        assert_eq!(
            adapter.state,
            InternalState::Error,
            "adapter should be in Error state after podman rm failure"
        );
    }
}
