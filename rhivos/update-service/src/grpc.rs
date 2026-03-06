use std::sync::Arc;

use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};

use crate::manager::{AdapterManager, ManagerError};
use crate::proto;

/// gRPC service implementation for UpdateService.
pub struct UpdateServiceImpl {
    pub(crate) manager: Arc<AdapterManager>,
}

impl UpdateServiceImpl {
    pub fn new(manager: Arc<AdapterManager>) -> Self {
        Self { manager }
    }
}

/// Map ManagerError to gRPC Status.
fn manager_error_to_status(err: ManagerError) -> Status {
    match err {
        ManagerError::NotFound(msg) => Status::not_found(format!("adapter not found: {msg}")),
        ManagerError::AlreadyExists(msg) => {
            Status::already_exists(format!("adapter already installed and running: {msg}"))
        }
        ManagerError::ChecksumMismatch { expected, actual } => Status::invalid_argument(format!(
            "checksum mismatch: expected {expected}, got {actual}"
        )),
        ManagerError::RegistryUnavailable(msg) => {
            Status::unavailable(format!("failed to pull image: {msg}"))
        }
        ManagerError::ContainerFailed(msg) => {
            Status::internal(format!("container failed to start: {msg}"))
        }
        ManagerError::InvalidTransition(msg) => {
            Status::internal(format!("invalid state transition: {msg}"))
        }
        ManagerError::Internal(msg) => Status::internal(format!("internal error: {msg}")),
    }
}

#[tonic::async_trait]
impl proto::update_service_server::UpdateService for UpdateServiceImpl {
    async fn install_adapter(
        &self,
        request: Request<proto::InstallAdapterRequest>,
    ) -> Result<Response<proto::InstallAdapterResponse>, Status> {
        let req = request.into_inner();
        let result = self
            .manager
            .install_adapter(&req.image_ref, &req.checksum_sha256)
            .await
            .map_err(manager_error_to_status)?;

        Ok(Response::new(proto::InstallAdapterResponse {
            job_id: result.job_id,
            adapter_id: result.adapter_id,
            state: i32::from(result.state),
        }))
    }

    type WatchAdapterStatesStream = std::pin::Pin<
        Box<dyn tokio_stream::Stream<Item = Result<proto::AdapterStateEvent, Status>> + Send>,
    >;

    async fn watch_adapter_states(
        &self,
        _request: Request<proto::WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        let rx = self.manager.subscribe_state_events();
        let stream = BroadcastStream::new(rx).filter_map(|result| match result {
            Ok(event) => Some(Ok(proto::AdapterStateEvent {
                adapter_id: event.adapter_id,
                old_state: i32::from(event.old_state),
                new_state: i32::from(event.new_state),
                timestamp: event.timestamp,
            })),
            Err(_) => None, // Skip lagged messages
        });

        Ok(Response::new(Box::pin(stream)))
    }

    async fn list_adapters(
        &self,
        _request: Request<proto::ListAdaptersRequest>,
    ) -> Result<Response<proto::ListAdaptersResponse>, Status> {
        let adapters = self.manager.list_adapters().await;
        let adapter_infos: Vec<proto::AdapterInfo> = adapters
            .into_iter()
            .map(|r| proto::AdapterInfo {
                adapter_id: r.adapter_id,
                image_ref: r.image_ref,
                state: i32::from(r.state),
            })
            .collect();

        Ok(Response::new(proto::ListAdaptersResponse {
            adapters: adapter_infos,
        }))
    }

    async fn remove_adapter(
        &self,
        request: Request<proto::RemoveAdapterRequest>,
    ) -> Result<Response<proto::RemoveAdapterResponse>, Status> {
        let req = request.into_inner();
        self.manager
            .remove_adapter(&req.adapter_id)
            .await
            .map_err(manager_error_to_status)?;

        Ok(Response::new(proto::RemoveAdapterResponse { success: true }))
    }

    async fn get_adapter_status(
        &self,
        request: Request<proto::GetAdapterStatusRequest>,
    ) -> Result<Response<proto::GetAdapterStatusResponse>, Status> {
        let req = request.into_inner();
        let record = self
            .manager
            .get_adapter_status(&req.adapter_id)
            .await
            .map_err(manager_error_to_status)?;

        Ok(Response::new(proto::GetAdapterStatusResponse {
            adapter_id: record.adapter_id,
            image_ref: record.image_ref,
            state: i32::from(record.state),
            error_message: record.error_message.unwrap_or_default(),
        }))
    }
}

#[cfg(test)]
#[path = "grpc_test.rs"]
mod tests;
