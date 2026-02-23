/// Generated proto types for the ParkingAdaptor service.
pub mod proto {
    pub mod adaptor {
        tonic::include_proto!("parking.adaptor.v1");
    }
}

use proto::adaptor::parking_adaptor_server::ParkingAdaptor;
use proto::adaptor::{
    GetRateRequest, GetRateResponse, GetStatusRequest, GetStatusResponse, StartSessionRequest,
    StartSessionResponse, StopSessionRequest, StopSessionResponse,
};
use tonic::{Request, Response, Status};

/// Stub implementation of the ParkingAdaptor gRPC service.
pub struct ParkingAdaptorImpl;

#[tonic::async_trait]
impl ParkingAdaptor for ParkingAdaptorImpl {
    async fn start_session(
        &self,
        _request: Request<StartSessionRequest>,
    ) -> Result<Response<StartSessionResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn stop_session(
        &self,
        _request: Request<StopSessionRequest>,
    ) -> Result<Response<StopSessionResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn get_status(
        &self,
        _request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn get_rate(
        &self,
        _request: Request<GetRateRequest>,
    ) -> Result<Response<GetRateResponse>, Status> {
        Err(Status::unimplemented("not yet implemented"))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        assert!(true, "parking-operator-adaptor skeleton compiles and tests run");
    }
}
