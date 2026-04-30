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

