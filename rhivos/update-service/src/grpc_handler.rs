/// gRPC service handler: wraps UpdateService with tonic protocol implementation.
///
/// Maps between protobuf types and internal domain types, handles streaming via
/// broadcast channel, and wires up the container monitor after adapters reach RUNNING.
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::broadcast;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::adapter::{AdapterState, AdapterStateEvent};
use crate::monitor::monitor_container;
use crate::podman::PodmanExecutor;
use crate::proto::update_service_server;
use crate::proto::{
    AdapterInfo, AdapterStateEvent as ProtoAdapterStateEvent, GetAdapterStatusRequest,
    InstallAdapterRequest, InstallAdapterResponse as ProtoInstallResponse, ListAdaptersRequest,
    ListAdaptersResponse, RemoveAdapterRequest, WatchAdapterStatesRequest,
};
use crate::service::{ServiceErrorCode, UpdateService};

// ────────────────────────────────────────────────────────────────────────────
// Helper conversions
// ────────────────────────────────────────────────────────────────────────────

fn to_status(err: crate::service::ServiceError) -> Status {
    match err.code {
        ServiceErrorCode::InvalidArgument => Status::invalid_argument(err.message),
        ServiceErrorCode::NotFound => Status::not_found(err.message),
        ServiceErrorCode::Internal => Status::internal(err.message),
    }
}

fn state_to_i32(s: &AdapterState) -> i32 {
    match s {
        AdapterState::Unknown => 0,
        AdapterState::Downloading => 1,
        AdapterState::Installing => 2,
        AdapterState::Running => 3,
        AdapterState::Stopped => 4,
        AdapterState::Error => 5,
        AdapterState::Offloading => 6,
    }
}

fn to_proto_event(event: &AdapterStateEvent) -> ProtoAdapterStateEvent {
    ProtoAdapterStateEvent {
        adapter_id: event.adapter_id.clone(),
        old_state: state_to_i32(&event.old_state),
        new_state: state_to_i32(&event.new_state),
        timestamp: event.timestamp as i64,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// gRPC service implementation
// ────────────────────────────────────────────────────────────────────────────

type StreamItem = Result<ProtoAdapterStateEvent, Status>;
type ResponseStream = Pin<Box<dyn futures::Stream<Item = StreamItem> + Send + 'static>>;

/// Tonic gRPC service handler for UPDATE_SERVICE.
pub struct UpdateServiceGrpc<P: PodmanExecutor + Send + Sync + 'static> {
    inner: Arc<UpdateService<P>>,
}

impl<P: PodmanExecutor + Send + Sync + 'static> UpdateServiceGrpc<P> {
    pub fn new(inner: Arc<UpdateService<P>>) -> Self {
        Self { inner }
    }
}

#[tonic::async_trait]
impl<P: PodmanExecutor + Send + Sync + 'static> update_service_server::UpdateService
    for UpdateServiceGrpc<P>
{
    type WatchAdapterStatesStream = ResponseStream;

    /// InstallAdapter: validate, pull, verify, run, return immediately with DOWNLOADING state.
    ///
    /// A background watcher task detects when the adapter reaches RUNNING state and
    /// then spawns the container monitor (podman wait → STOPPED/ERROR transitions).
    async fn install_adapter(
        &self,
        request: Request<InstallAdapterRequest>,
    ) -> Result<Response<ProtoInstallResponse>, Status> {
        let req = request.into_inner();

        // Subscribe BEFORE calling install_adapter so we don't miss RUNNING event
        // even if the async install task runs concurrently on another thread.
        let mut event_rx = self.inner.watch_adapter_states();

        let resp = self
            .inner
            .install_adapter(&req.image_ref, &req.checksum_sha256)
            .await
            .map_err(to_status)?;

        let adapter_id = resp.adapter_id.clone();
        let state = Arc::clone(&self.inner.state);
        let podman = Arc::clone(&self.inner.podman);

        // Spawn a watcher that starts the container monitor once the adapter is RUNNING.
        // The monitor handles exit detection (RUNNING → STOPPED or ERROR).
        tokio::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(event) if event.adapter_id == adapter_id => match event.new_state {
                        AdapterState::Running => {
                            let state_c = Arc::clone(&state);
                            let podman_c = Arc::clone(&podman);
                            let id_c = adapter_id.clone();
                            tokio::spawn(async move {
                                monitor_container(id_c, String::new(), state_c, podman_c).await;
                            });
                            break;
                        }
                        // Install failed or adapter removed — no monitor needed.
                        AdapterState::Error => break,
                        _ => {} // keep watching through DOWNLOADING, INSTALLING
                    },
                    Ok(_) => {}                                           // different adapter
                    Err(broadcast::error::RecvError::Lagged(_)) => {}    // skip lagged events
                    Err(broadcast::error::RecvError::Closed) => break,   // channel shut down
                }
            }
        });

        Ok(Response::new(ProtoInstallResponse {
            job_id: resp.job_id,
            adapter_id: resp.adapter_id,
            state: state_to_i32(&resp.state),
        }))
    }

    /// WatchAdapterStates: fan out state events to this gRPC stream subscriber.
    async fn watch_adapter_states(
        &self,
        _request: Request<WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        let broadcast_rx = self.inner.watch_adapter_states();
        let (tx, rx) = tokio::sync::mpsc::channel::<StreamItem>(100);

        // Bridge broadcast receiver → mpsc sender so tonic can stream it.
        tokio::spawn(async move {
            let mut broadcast_rx = broadcast_rx;
            loop {
                match broadcast_rx.recv().await {
                    Ok(event) => {
                        let proto_event = to_proto_event(&event);
                        if tx.send(Ok(proto_event)).await.is_err() {
                            break; // client disconnected
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WatchAdapterStates subscriber lagged by {n} events");
                        // continue — future events are still receivable
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    /// ListAdapters: return all known adapters with their current states.
    async fn list_adapters(
        &self,
        _request: Request<ListAdaptersRequest>,
    ) -> Result<Response<ListAdaptersResponse>, Status> {
        let adapters = self.inner.list_adapters();
        let proto_adapters: Vec<AdapterInfo> = adapters
            .iter()
            .map(|a| AdapterInfo {
                adapter_id: a.adapter_id.clone(),
                image_ref: a.image_ref.clone(),
                state: state_to_i32(&a.state),
            })
            .collect();
        Ok(Response::new(ListAdaptersResponse {
            adapters: proto_adapters,
        }))
    }

    /// RemoveAdapter: stop (if running) + rm container + rmi image, remove from state.
    async fn remove_adapter(
        &self,
        request: Request<RemoveAdapterRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        self.inner
            .remove_adapter(&req.adapter_id)
            .await
            .map_err(to_status)?;
        Ok(Response::new(()))
    }

    /// GetAdapterStatus: return current state of specified adapter.
    async fn get_adapter_status(
        &self,
        request: Request<GetAdapterStatusRequest>,
    ) -> Result<Response<AdapterInfo>, Status> {
        let req = request.into_inner();
        let entry = self
            .inner
            .get_adapter_status(&req.adapter_id)
            .map_err(to_status)?;
        Ok(Response::new(AdapterInfo {
            adapter_id: entry.adapter_id,
            image_ref: entry.image_ref,
            state: state_to_i32(&entry.state),
        }))
    }
}
