pub mod adapter_manager;
pub mod checksum;
pub mod config;

/// Generated proto types for the UpdateService.
///
/// Module hierarchy mirrors the proto package structure so that
/// cross-package references (e.g. `super::super::common::v1::AdapterState`)
/// resolve correctly.
pub mod parking {
    pub mod common {
        pub mod v1 {
            tonic::include_proto!("parking.common.v1");
        }
    }
    pub mod update {
        pub mod v1 {
            tonic::include_proto!("parking.update.v1");
        }
    }
}

use parking::update::v1::update_service_server::UpdateService;
use parking::update::v1::{
    AdapterStateEvent, GetAdapterStatusRequest, GetAdapterStatusResponse, InstallAdapterRequest,
    InstallAdapterResponse, ListAdaptersRequest, ListAdaptersResponse, RemoveAdapterRequest,
    RemoveAdapterResponse, WatchAdapterStatesRequest,
};
use tonic::{Request, Response, Status};

/// Stub implementation of the UpdateService gRPC service.
pub struct UpdateServiceImpl;

#[tonic::async_trait]
impl UpdateService for UpdateServiceImpl {
    async fn install_adapter(
        &self,
        _request: Request<InstallAdapterRequest>,
    ) -> Result<Response<InstallAdapterResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    type WatchAdapterStatesStream =
        tokio_stream::wrappers::ReceiverStream<Result<AdapterStateEvent, Status>>;

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
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true, "update-service skeleton compiles and tests run");
    }
}
