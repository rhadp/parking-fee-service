use std::sync::Arc;
use tokio::sync::broadcast;
use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;

use crate::manager::{AdapterManager, ManagerError};
use crate::state::AdapterState as ManagerState;

pub mod proto {
    tonic::include_proto!("update_service.v1");
}

use proto::update_service_server::{UpdateService, UpdateServiceServer};
use proto::*;

/// Convert our internal AdapterState to the protobuf i32 value.
fn state_to_proto(state: ManagerState) -> i32 {
    match state {
        ManagerState::Unknown => AdapterState::Unknown as i32,
        ManagerState::Downloading => AdapterState::Downloading as i32,
        ManagerState::Installing => AdapterState::Installing as i32,
        ManagerState::Running => AdapterState::Running as i32,
        ManagerState::Stopped => AdapterState::Stopped as i32,
        ManagerState::Error => AdapterState::Error as i32,
        ManagerState::Offloading => AdapterState::Offloading as i32,
    }
}

/// Map a ManagerError to a tonic Status.
fn manager_err_to_status(e: ManagerError) -> Status {
    match e {
        ManagerError::NotFound(msg) => Status::not_found(msg),
        ManagerError::AlreadyExists(msg) => Status::already_exists(msg),
        ManagerError::ChecksumMismatch { expected, actual } => Status::invalid_argument(format!(
            "checksum mismatch: expected {expected}, got {actual}"
        )),
        ManagerError::RegistryUnavailable(msg) => Status::unavailable(msg),
        ManagerError::ContainerStartFailed(msg) => Status::internal(msg),
        ManagerError::InvalidTransition { from, to } => {
            Status::internal(format!("invalid transition {:?} -> {:?}", from, to))
        }
        ManagerError::Internal(msg) => Status::internal(msg),
    }
}

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
            .map_err(manager_err_to_status)?;

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
        let mut rx = self.manager.subscribe_state_events();
        let (tx, inner_rx) = tokio::sync::mpsc::channel(128);

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
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
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(inner_rx)))
    }

    async fn list_adapters(
        &self,
        _request: Request<ListAdaptersRequest>,
    ) -> Result<Response<ListAdaptersResponse>, Status> {
        let records = self.manager.list_adapters().await;
        let adapters = records
            .into_iter()
            .map(|r| AdapterInfo {
                adapter_id: r.adapter_id,
                image_ref: r.image_ref,
                state: state_to_proto(r.state),
            })
            .collect();
        Ok(Response::new(ListAdaptersResponse { adapters }))
    }

    async fn remove_adapter(
        &self,
        request: Request<RemoveAdapterRequest>,
    ) -> Result<Response<RemoveAdapterResponse>, Status> {
        let adapter_id = request.into_inner().adapter_id;
        self.manager
            .remove_adapter(&adapter_id)
            .await
            .map_err(manager_err_to_status)?;
        Ok(Response::new(RemoveAdapterResponse { success: true }))
    }

    async fn get_adapter_status(
        &self,
        request: Request<GetAdapterStatusRequest>,
    ) -> Result<Response<GetAdapterStatusResponse>, Status> {
        let adapter_id = request.into_inner().adapter_id;
        let record = self
            .manager
            .get_adapter_status(&adapter_id)
            .await
            .map_err(manager_err_to_status)?;
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
