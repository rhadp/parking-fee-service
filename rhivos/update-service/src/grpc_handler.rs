use std::sync::Arc;

use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};

use crate::adapter::AdapterState as ModelState;
use crate::podman::PodmanExecutor;
use crate::proto::update::{
    update_service_server::UpdateService, AdapterInfo, AdapterStateEvent, GetAdapterStatusRequest,
    GetAdapterStatusResponse, InstallAdapterRequest, InstallAdapterResponse, ListAdaptersRequest,
    ListAdaptersResponse, RemoveAdapterRequest, RemoveAdapterResponse, WatchAdapterStatesRequest,
};
use crate::service::{ServiceError, UpdateService as CoreService};

/// Convert the Rust model `AdapterState` to the proto enum integer.
fn model_to_proto_state(s: &ModelState) -> i32 {
    match s {
        ModelState::Unknown => 0,
        ModelState::Downloading => 1,
        ModelState::Installing => 2,
        ModelState::Running => 3,
        ModelState::Stopped => 4,
        ModelState::Error => 5,
        ModelState::Offloading => 6,
    }
}

/// Map a `ServiceError` to a tonic `Status`.
fn service_err_to_status(e: ServiceError) -> Status {
    match e {
        ServiceError::InvalidArgument(msg) => Status::invalid_argument(msg),
        ServiceError::NotFound(msg) => Status::not_found(msg),
        ServiceError::Internal(msg) => Status::internal(msg),
    }
}

/// gRPC handler wrapping the core `UpdateService`.
pub struct GrpcHandler<P: PodmanExecutor + Send + Sync + 'static> {
    core: Arc<CoreService<P>>,
}

impl<P: PodmanExecutor + Send + Sync + 'static> GrpcHandler<P> {
    pub fn new(core: Arc<CoreService<P>>) -> Self {
        Self { core }
    }
}

#[tonic::async_trait]
impl<P: PodmanExecutor + Send + Sync + 'static> UpdateService for GrpcHandler<P> {
    // ----- InstallAdapter -----
    async fn install_adapter(
        &self,
        request: Request<InstallAdapterRequest>,
    ) -> Result<Response<InstallAdapterResponse>, Status> {
        let req = request.into_inner();
        let resp = self
            .core
            .install_adapter(&req.image_ref, &req.checksum_sha256)
            .await
            .map_err(service_err_to_status)?;

        Ok(Response::new(InstallAdapterResponse {
            job_id: resp.job_id,
            adapter_id: resp.adapter_id,
            state: model_to_proto_state(&resp.state),
        }))
    }

    // ----- WatchAdapterStates -----
    type WatchAdapterStatesStream =
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<AdapterStateEvent, Status>> + Send>>;

    async fn watch_adapter_states(
        &self,
        _request: Request<WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        let receiver = self.core.subscribe();
        let stream = BroadcastStream::new(receiver).filter_map(|msg| match msg {
            Ok(event) => {
                let proto_event = AdapterStateEvent {
                    adapter_id: event.adapter_id,
                    old_state: model_to_proto_state(&event.old_state),
                    new_state: model_to_proto_state(&event.new_state),
                    timestamp: event.timestamp as i64,
                };
                Some(Ok(proto_event))
            }
            Err(_lagged) => {
                // Subscriber was too slow; skip lagged events silently.
                None
            }
        });

        Ok(Response::new(Box::pin(stream)))
    }

    // ----- ListAdapters -----
    async fn list_adapters(
        &self,
        _request: Request<ListAdaptersRequest>,
    ) -> Result<Response<ListAdaptersResponse>, Status> {
        let adapters = self.core.list_adapters();
        let proto_adapters = adapters
            .into_iter()
            .map(|entry| AdapterInfo {
                adapter_id: entry.adapter_id,
                image_ref: entry.image_ref,
                state: model_to_proto_state(&entry.state),
            })
            .collect();

        Ok(Response::new(ListAdaptersResponse {
            adapters: proto_adapters,
        }))
    }

    // ----- RemoveAdapter -----
    async fn remove_adapter(
        &self,
        request: Request<RemoveAdapterRequest>,
    ) -> Result<Response<RemoveAdapterResponse>, Status> {
        let adapter_id = request.into_inner().adapter_id;
        self.core
            .remove_adapter(&adapter_id)
            .await
            .map_err(service_err_to_status)?;

        Ok(Response::new(RemoveAdapterResponse {
            success: true,
            message: "adapter removed".to_string(),
        }))
    }

    // ----- GetAdapterStatus -----
    async fn get_adapter_status(
        &self,
        request: Request<GetAdapterStatusRequest>,
    ) -> Result<Response<GetAdapterStatusResponse>, Status> {
        let adapter_id = request.into_inner().adapter_id;
        let entry = self
            .core
            .get_adapter_status(&adapter_id)
            .map_err(service_err_to_status)?;

        Ok(Response::new(GetAdapterStatusResponse {
            adapter: Some(AdapterInfo {
                adapter_id: entry.adapter_id,
                image_ref: entry.image_ref,
                state: model_to_proto_state(&entry.state),
            }),
        }))
    }
}
