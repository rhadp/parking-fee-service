use std::sync::Arc;
use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;

use crate::manager::AdapterManager;

pub mod proto {
    tonic::include_proto!("update_service.v1");
}

use proto::update_service_server::{UpdateService, UpdateServiceServer};
use proto::*;

/// gRPC service implementation for UpdateService.
pub struct UpdateServiceImpl {
    pub manager: Arc<AdapterManager>,
}

impl UpdateServiceImpl {
    pub fn new(manager: Arc<AdapterManager>) -> Self {
        Self { manager }
    }

    pub fn into_server(self) -> UpdateServiceServer<Self> {
        UpdateServiceServer::new(self)
    }
}

#[tonic::async_trait]
impl UpdateService for UpdateServiceImpl {
    async fn install_adapter(
        &self,
        _request: Request<InstallAdapterRequest>,
    ) -> Result<Response<InstallAdapterResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    type WatchAdapterStatesStream = ReceiverStream<Result<AdapterStateEvent, Status>>;

    async fn watch_adapter_states(
        &self,
        _request: Request<WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn list_adapters(
        &self,
        _request: Request<ListAdaptersRequest>,
    ) -> Result<Response<ListAdaptersResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn remove_adapter(
        &self,
        _request: Request<RemoveAdapterRequest>,
    ) -> Result<Response<RemoveAdapterResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn get_adapter_status(
        &self,
        _request: Request<GetAdapterStatusRequest>,
    ) -> Result<Response<GetAdapterStatusResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }
}

#[cfg(test)]
#[path = "grpc_test.rs"]
mod grpc_test;
