use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::broadcast;
use tonic::{Request, Response, Status};

use crate::adapter::AdapterStateEvent;
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
        _state_mgr: Arc<StateManager>,
        _podman: Arc<dyn PodmanExecutor>,
        _adapter_id: String,
        _image_ref: String,
        _checksum_sha256: String,
    ) {
        todo!("install_adapter_workflow not yet implemented")
    }
}

type EventStream =
    Pin<Box<dyn futures::Stream<Item = Result<proto::AdapterStateEvent, Status>> + Send>>;

#[tonic::async_trait]
impl proto::update_service_server::UpdateService for UpdateServiceImpl {
    async fn install_adapter(
        &self,
        _request: Request<proto::InstallAdapterRequest>,
    ) -> Result<Response<proto::InstallAdapterResponse>, Status> {
        todo!("install_adapter RPC not yet implemented")
    }

    type WatchAdapterStatesStream = EventStream;

    async fn watch_adapter_states(
        &self,
        _request: Request<proto::WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        todo!("watch_adapter_states RPC not yet implemented")
    }

    async fn list_adapters(
        &self,
        _request: Request<proto::ListAdaptersRequest>,
    ) -> Result<Response<proto::ListAdaptersResponse>, Status> {
        todo!("list_adapters RPC not yet implemented")
    }

    async fn remove_adapter(
        &self,
        _request: Request<proto::RemoveAdapterRequest>,
    ) -> Result<Response<proto::RemoveAdapterResponse>, Status> {
        todo!("remove_adapter RPC not yet implemented")
    }

    async fn get_adapter_status(
        &self,
        _request: Request<proto::GetAdapterStatusRequest>,
    ) -> Result<Response<proto::GetAdapterStatusResponse>, Status> {
        todo!("get_adapter_status RPC not yet implemented")
    }
}
