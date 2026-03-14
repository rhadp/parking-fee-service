//! gRPC handler: implements the generated `UpdateService` tonic trait.
//!
//! Each RPC delegates to the appropriate `service` or `state` module
//! function, then maps internal types and errors to proto/gRPC types.

use std::pin::Pin;
use std::sync::Arc;

use futures::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;
use tonic::{Request, Response, Status};

use crate::container::ContainerRuntime;
use crate::model::{self, AdapterState};
use crate::proto::common::{AdapterInfo as PbAdapterInfo, AdapterState as PbAdapterState};
use crate::proto::updateservice::{
    update_service_server::UpdateService, AdapterStateEvent as PbAdapterStateEvent,
    GetAdapterStatusRequest, GetAdapterStatusResponse, InstallAdapterRequest,
    InstallAdapterResponse, ListAdaptersRequest, ListAdaptersResponse, RemoveAdapterRequest,
    RemoveAdapterResponse, WatchAdapterStatesRequest,
};
use crate::service::{self, ServiceError};
use crate::state::StateManager;

// ---------------------------------------------------------------------------
// Type alias for the server-streaming response type
// ---------------------------------------------------------------------------

type WatchStream =
    Pin<Box<dyn futures::Stream<Item = Result<PbAdapterStateEvent, Status>> + Send + 'static>>;

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

/// Convert internal `AdapterState` to the proto enum's i32 wire value.
fn model_state_to_proto(s: &AdapterState) -> i32 {
    match s {
        AdapterState::Unknown => PbAdapterState::Unknown as i32,
        AdapterState::Downloading => PbAdapterState::Downloading as i32,
        AdapterState::Installing => PbAdapterState::Installing as i32,
        AdapterState::Running => PbAdapterState::Running as i32,
        AdapterState::Stopped => PbAdapterState::Stopped as i32,
        AdapterState::Error => PbAdapterState::Error as i32,
        AdapterState::Offloading => PbAdapterState::Offloading as i32,
    }
}

/// Map `ServiceError` to a `tonic::Status` with the appropriate gRPC code.
fn service_err_to_status(e: ServiceError) -> Status {
    match e {
        ServiceError::InvalidArgument(m) => Status::invalid_argument(m),
        ServiceError::Unavailable(m) => Status::unavailable(m),
        ServiceError::FailedPrecondition(m) => Status::failed_precondition(m),
        ServiceError::Internal(m) => Status::internal(m),
        ServiceError::NotFound(m) => Status::not_found(m),
    }
}

/// Convert an internal `AdapterInfo` to its proto representation.
fn adapter_info_to_proto(info: &model::AdapterInfo) -> PbAdapterInfo {
    PbAdapterInfo {
        adapter_id: info.adapter_id.clone(),
        operator_id: String::new(), // not tracked by this service
        image_ref: info.image_ref.clone(),
        checksum_sha256: info.checksum.clone(),
        version: String::new(), // not tracked by this service
        state: model_state_to_proto(&info.state),
    }
}

// ---------------------------------------------------------------------------
// UpdateServiceImpl
// ---------------------------------------------------------------------------

/// gRPC service implementation.  Holds shared references to the state manager
/// and the container runtime so that each RPC can delegate to them.
pub struct UpdateServiceImpl {
    manager: Arc<StateManager>,
    runtime: Arc<dyn ContainerRuntime>,
}

impl UpdateServiceImpl {
    pub fn new(manager: Arc<StateManager>, runtime: Arc<dyn ContainerRuntime>) -> Self {
        Self { manager, runtime }
    }
}

#[tonic::async_trait]
impl UpdateService for UpdateServiceImpl {
    // -----------------------------------------------------------------------
    // InstallAdapter — REQ-1.1, REQ-1.E1..E4, REQ-2.1, REQ-2.2
    // -----------------------------------------------------------------------

    async fn install_adapter(
        &self,
        request: Request<InstallAdapterRequest>,
    ) -> Result<Response<InstallAdapterResponse>, Status> {
        let req = request.into_inner();

        let resp = service::install_adapter(
            Arc::clone(&self.manager),
            Arc::clone(&self.runtime),
            &req.image_ref,
            &req.checksum_sha256,
        )
        .await
        .map_err(service_err_to_status)?;

        Ok(Response::new(InstallAdapterResponse {
            job_id: resp.job_id,
            adapter_id: resp.adapter_id,
            // The response reflects the *initial* state at record creation
            // (always DOWNLOADING), matching REQ-1.1.
            state: model_state_to_proto(&resp.initial_state),
        }))
    }

    // -----------------------------------------------------------------------
    // WatchAdapterStates — REQ-3.1, REQ-3.2, REQ-3.3
    // -----------------------------------------------------------------------

    type WatchAdapterStatesStream = WatchStream;

    async fn watch_adapter_states(
        &self,
        _request: Request<WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        let rx = self.manager.subscribe();

        // Convert broadcast::Receiver → Stream<Item = Result<PbAdapterStateEvent, Status>>.
        // BroadcastStream wraps the receiver and yields Ok(event) or Err(Lagged).
        // We silently discard lagged errors (slow consumers miss intermediate events)
        // and map each valid event to its proto representation.
        let stream = BroadcastStream::new(rx).filter_map(|result| async move {
            match result {
                Ok(event) => Some(Ok(PbAdapterStateEvent {
                    adapter_id: event.adapter_id,
                    old_state: model_state_to_proto(&event.old_state),
                    new_state: model_state_to_proto(&event.new_state),
                    timestamp: event.timestamp,
                })),
                // Lagged receiver: skip the gap and continue streaming
                Err(_lagged) => None,
            }
        });

        Ok(Response::new(Box::pin(stream)))
    }

    // -----------------------------------------------------------------------
    // ListAdapters — REQ-4.1
    // -----------------------------------------------------------------------

    async fn list_adapters(
        &self,
        _request: Request<ListAdaptersRequest>,
    ) -> Result<Response<ListAdaptersResponse>, Status> {
        let adapters = self.manager.list();
        let pb_adapters = adapters.iter().map(adapter_info_to_proto).collect();
        Ok(Response::new(ListAdaptersResponse {
            adapters: pb_adapters,
        }))
    }

    // -----------------------------------------------------------------------
    // RemoveAdapter — REQ-5.1..5.3, REQ-5.E1, REQ-5.E2
    // -----------------------------------------------------------------------

    async fn remove_adapter(
        &self,
        request: Request<RemoveAdapterRequest>,
    ) -> Result<Response<RemoveAdapterResponse>, Status> {
        let req = request.into_inner();

        service::remove_adapter(
            Arc::clone(&self.manager),
            Arc::clone(&self.runtime),
            &req.adapter_id,
        )
        .await
        .map_err(service_err_to_status)?;

        Ok(Response::new(RemoveAdapterResponse { success: true }))
    }

    // -----------------------------------------------------------------------
    // GetAdapterStatus — REQ-4.2, REQ-4.E1
    // -----------------------------------------------------------------------

    async fn get_adapter_status(
        &self,
        request: Request<GetAdapterStatusRequest>,
    ) -> Result<Response<GetAdapterStatusResponse>, Status> {
        let req = request.into_inner();

        let info = self.manager.get(&req.adapter_id).ok_or_else(|| {
            Status::not_found(format!("adapter '{}' not found", req.adapter_id))
        })?;

        Ok(Response::new(GetAdapterStatusResponse {
            adapter: Some(adapter_info_to_proto(&info)),
        }))
    }
}
