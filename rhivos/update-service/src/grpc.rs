use std::sync::Arc;
use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;

use crate::manager::{AdapterManager, ManagerError};
use crate::state::AdapterState;

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

/// Map internal AdapterState to protobuf AdapterState i32 value.
fn state_to_proto(state: AdapterState) -> i32 {
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

/// Map ManagerError to appropriate gRPC Status.
fn manager_error_to_status(err: ManagerError) -> Status {
    match err {
        ManagerError::NotFound(msg) => Status::not_found(msg),
        ManagerError::AlreadyExists(msg) => Status::already_exists(msg),
        ManagerError::ChecksumMismatch { expected, actual } => {
            Status::invalid_argument(format!(
                "checksum mismatch: expected {}, got {}",
                expected, actual
            ))
        }
        ManagerError::RegistryUnavailable(msg) => Status::unavailable(msg),
        ManagerError::ContainerStartFailed(msg) => Status::internal(msg),
        ManagerError::InvalidTransition { from, to } => {
            Status::internal(format!("invalid state transition: {:?} -> {:?}", from, to))
        }
        ManagerError::Internal(msg) => Status::internal(msg),
    }
}

#[tonic::async_trait]
impl UpdateService for UpdateServiceImpl {
    async fn install_adapter(
        &self,
        request: Request<InstallAdapterRequest>,
    ) -> Result<Response<InstallAdapterResponse>, Status> {
        let req = request.into_inner();
        let result = self
            .manager
            .install_adapter(&req.image_ref, &req.checksum_sha256)
            .await
            .map_err(manager_error_to_status)?;

        Ok(Response::new(InstallAdapterResponse {
            job_id: result.job_id,
            adapter_id: result.adapter_id,
            state: state_to_proto(result.state),
        }))
    }

    type WatchAdapterStatesStream = ReceiverStream<Result<AdapterStateEvent, Status>>;

    async fn watch_adapter_states(
        &self,
        _request: Request<WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        let mut broadcast_rx = self.manager.subscribe_state_events();
        let (tx, rx) = tokio::sync::mpsc::channel(256);

        tokio::spawn(async move {
            while let Ok(event) = broadcast_rx.recv().await {
                let proto_event = AdapterStateEvent {
                    adapter_id: event.adapter_id,
                    old_state: state_to_proto(event.old_state),
                    new_state: state_to_proto(event.new_state),
                    timestamp: event.timestamp,
                };
                if tx.send(Ok(proto_event)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn list_adapters(
        &self,
        _request: Request<ListAdaptersRequest>,
    ) -> Result<Response<ListAdaptersResponse>, Status> {
        let adapters = self.manager.list_adapters().await;
        let adapter_infos: Vec<AdapterInfo> = adapters
            .into_iter()
            .map(|a| AdapterInfo {
                adapter_id: a.adapter_id,
                image_ref: a.image_ref,
                state: state_to_proto(a.state),
            })
            .collect();

        Ok(Response::new(ListAdaptersResponse {
            adapters: adapter_infos,
        }))
    }

    async fn remove_adapter(
        &self,
        request: Request<RemoveAdapterRequest>,
    ) -> Result<Response<RemoveAdapterResponse>, Status> {
        let req = request.into_inner();
        self.manager
            .remove_adapter(&req.adapter_id)
            .await
            .map_err(manager_error_to_status)?;

        Ok(Response::new(RemoveAdapterResponse { success: true }))
    }

    async fn get_adapter_status(
        &self,
        request: Request<GetAdapterStatusRequest>,
    ) -> Result<Response<GetAdapterStatusResponse>, Status> {
        let req = request.into_inner();
        let record = self
            .manager
            .get_adapter_status(&req.adapter_id)
            .await
            .map_err(manager_error_to_status)?;

        Ok(Response::new(GetAdapterStatusResponse {
            adapter_id: record.adapter_id,
            image_ref: record.image_ref,
            state: state_to_proto(record.state),
            error_message: record.error_message.unwrap_or_default(),
        }))
    }
}

#[cfg(test)]
#[path = "grpc_test.rs"]
mod grpc_test;
