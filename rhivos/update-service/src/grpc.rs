use crate::adapter::AdapterState as InternalAdapterState;
use crate::config::Config;
use crate::monitor::monitor_container;
use crate::podman::PodmanExecutor;
use crate::proto::{
    update_service_server::UpdateService, AdapterInfo, AdapterState as ProtoAdapterState,
    AdapterStateEvent as ProtoAdapterStateEvent, GetAdapterStatusRequest,
    GetAdapterStatusResponse, InstallAdapterRequest, InstallAdapterResponse,
    ListAdaptersRequest, ListAdaptersResponse, RemoveAdapterRequest, RemoveAdapterResponse,
    WatchAdapterStatesRequest,
};
use crate::service::{ServiceError, UpdateServiceHandler};
use crate::state::StateManager;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

/// Convert internal AdapterState to protobuf AdapterState.
fn to_proto_state(state: &InternalAdapterState) -> i32 {
    match state {
        InternalAdapterState::Unknown => ProtoAdapterState::Unknown as i32,
        InternalAdapterState::Downloading => ProtoAdapterState::Downloading as i32,
        InternalAdapterState::Installing => ProtoAdapterState::Installing as i32,
        InternalAdapterState::Running => ProtoAdapterState::Running as i32,
        InternalAdapterState::Stopped => ProtoAdapterState::Stopped as i32,
        InternalAdapterState::Error => ProtoAdapterState::Error as i32,
        InternalAdapterState::Offloading => ProtoAdapterState::Offloading as i32,
    }
}

/// Convert a ServiceError to a tonic Status.
fn service_error_to_status(err: ServiceError) -> Status {
    match err {
        ServiceError::InvalidArgument(msg) => Status::invalid_argument(msg),
        ServiceError::NotFound(msg) => Status::not_found(msg),
        ServiceError::Internal(msg) => Status::internal(msg),
    }
}

/// gRPC service implementation wrapping UpdateServiceHandler.
pub struct GrpcUpdateService {
    handler: Arc<UpdateServiceHandler>,
    state_mgr: Arc<StateManager>,
    podman: Arc<dyn PodmanExecutor>,
}

impl GrpcUpdateService {
    pub fn new(
        state_mgr: Arc<StateManager>,
        podman: Arc<dyn PodmanExecutor>,
        config: Config,
    ) -> Self {
        let handler = Arc::new(UpdateServiceHandler::new(
            state_mgr.clone(),
            podman.clone(),
            config,
        ));
        Self {
            handler,
            state_mgr,
            podman,
        }
    }
}

#[tonic::async_trait]
impl UpdateService for GrpcUpdateService {
    async fn install_adapter(
        &self,
        request: Request<InstallAdapterRequest>,
    ) -> Result<Response<InstallAdapterResponse>, Status> {
        let req = request.into_inner();

        let result = self
            .handler
            .install_adapter(&req.image_ref, &req.checksum_sha256)
            .await
            .map_err(service_error_to_status)?;

        // Spawn container monitor for this adapter
        let adapter_id = result.adapter_id.clone();
        let state_mgr = self.state_mgr.clone();
        let podman = self.podman.clone();
        tokio::spawn(async move {
            // Wait briefly for the install to complete before monitoring
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            // Only monitor if the adapter reached RUNNING state
            if let Some(entry) = state_mgr.get_adapter(&adapter_id) {
                if entry.state == InternalAdapterState::Running {
                    monitor_container(&adapter_id, &state_mgr, &podman).await;
                }
            }
        });

        let resp = InstallAdapterResponse {
            job_id: result.job_id,
            adapter_id: result.adapter_id,
            state: to_proto_state(&result.state),
        };

        Ok(Response::new(resp))
    }

    type WatchAdapterStatesStream = ReceiverStream<Result<ProtoAdapterStateEvent, Status>>;

    async fn watch_adapter_states(
        &self,
        _request: Request<WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        let mut broadcast_rx = self.state_mgr.subscribe();
        let (tx, rx) = tokio::sync::mpsc::channel(128);

        tokio::spawn(async move {
            loop {
                match broadcast_rx.recv().await {
                    Ok(event) => {
                        let proto_event = ProtoAdapterStateEvent {
                            adapter_id: event.adapter_id,
                            old_state: to_proto_state(&event.old_state),
                            new_state: to_proto_state(&event.new_state),
                            timestamp: event.timestamp as i64,
                        };
                        if tx.send(Ok(proto_event)).await.is_err() {
                            // Client disconnected
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WatchAdapterStates subscriber lagged by {n} events");
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn list_adapters(
        &self,
        _request: Request<ListAdaptersRequest>,
    ) -> Result<Response<ListAdaptersResponse>, Status> {
        let adapters = self.handler.list_adapters();
        let adapter_infos: Vec<AdapterInfo> = adapters
            .into_iter()
            .map(|a| AdapterInfo {
                adapter_id: a.adapter_id,
                image_ref: a.image_ref,
                state: to_proto_state(&a.state),
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

        self.handler
            .remove_adapter(&req.adapter_id)
            .await
            .map_err(service_error_to_status)?;

        Ok(Response::new(RemoveAdapterResponse {
            success: true,
            message: String::new(),
        }))
    }

    async fn get_adapter_status(
        &self,
        request: Request<GetAdapterStatusRequest>,
    ) -> Result<Response<GetAdapterStatusResponse>, Status> {
        let req = request.into_inner();

        let entry = self
            .handler
            .get_adapter_status(&req.adapter_id)
            .map_err(service_error_to_status)?;

        let info = AdapterInfo {
            adapter_id: entry.adapter_id,
            image_ref: entry.image_ref,
            state: to_proto_state(&entry.state),
        };

        Ok(Response::new(GetAdapterStatusResponse {
            adapter: Some(info),
        }))
    }
}
