use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::manager::AdapterManager;
use crate::proto;

/// gRPC service implementation for UpdateService.
pub struct UpdateServiceImpl {
    pub(crate) _manager: Arc<AdapterManager>,
}

impl UpdateServiceImpl {
    pub fn new(manager: Arc<AdapterManager>) -> Self {
        Self { _manager: manager }
    }
}

#[tonic::async_trait]
impl proto::update_service_server::UpdateService for UpdateServiceImpl {
    async fn install_adapter(
        &self,
        _request: Request<proto::InstallAdapterRequest>,
    ) -> Result<Response<proto::InstallAdapterResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    type WatchAdapterStatesStream =
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<proto::AdapterStateEvent, Status>> + Send>>;

    async fn watch_adapter_states(
        &self,
        _request: Request<proto::WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn list_adapters(
        &self,
        _request: Request<proto::ListAdaptersRequest>,
    ) -> Result<Response<proto::ListAdaptersResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn remove_adapter(
        &self,
        _request: Request<proto::RemoveAdapterRequest>,
    ) -> Result<Response<proto::RemoveAdapterResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn get_adapter_status(
        &self,
        _request: Request<proto::GetAdapterStatusRequest>,
    ) -> Result<Response<proto::GetAdapterStatusResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }
}

#[cfg(test)]
#[path = "grpc_test.rs"]
mod tests;
