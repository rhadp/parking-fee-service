use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::session::SessionManager;

/// Generated protobuf types from parking_adaptor.proto.
pub mod pb {
    tonic::include_proto!("parking_adaptor");
}

/// gRPC service implementation for the ParkingAdaptor.
/// Stub: RPC handlers will be implemented in task group 2.
#[allow(dead_code)]
pub struct ParkingAdaptorService {
    session: Arc<Mutex<SessionManager>>,
    zone_id: String,
}

impl ParkingAdaptorService {
    pub fn new(session: Arc<Mutex<SessionManager>>, zone_id: String) -> Self {
        Self { session, zone_id }
    }
}

#[tonic::async_trait]
impl pb::parking_adaptor_server::ParkingAdaptor for ParkingAdaptorService {
    async fn start_session(
        &self,
        _request: Request<pb::StartSessionRequest>,
    ) -> Result<Response<pb::StartSessionResponse>, Status> {
        // Stub: not yet implemented
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn stop_session(
        &self,
        _request: Request<pb::StopSessionRequest>,
    ) -> Result<Response<pb::StopSessionResponse>, Status> {
        // Stub: not yet implemented
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn get_status(
        &self,
        _request: Request<pb::GetStatusRequest>,
    ) -> Result<Response<pb::GetStatusResponse>, Status> {
        // Stub: not yet implemented
        Err(Status::unimplemented("not yet implemented"))
    }

    async fn get_rate(
        &self,
        _request: Request<pb::GetRateRequest>,
    ) -> Result<Response<pb::GetRateResponse>, Status> {
        // Stub: not yet implemented
        Err(Status::unimplemented("not yet implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pb::parking_adaptor_server::ParkingAdaptor;

    /// Helper to create a service instance for testing.
    fn test_service(zone_id: &str) -> ParkingAdaptorService {
        let session = Arc::new(Mutex::new(SessionManager::new()));
        ParkingAdaptorService::new(session, zone_id.to_string())
    }

    /// TS-08-5: GetStatus returns idle state when no session is active.
    #[tokio::test]
    async fn test_get_status_idle() {
        let svc = test_service("zone-demo-1");
        let request = Request::new(pb::GetStatusRequest {});
        let response = svc.get_status(request).await;
        assert!(response.is_ok(), "GetStatus should succeed");
        let resp = response.unwrap().into_inner();
        assert_eq!(resp.state, "idle");
        assert!(resp.session_id.is_empty(), "session_id should be empty when idle");
    }

    /// TS-08-6: GetRate returns parking rate information.
    #[tokio::test]
    async fn test_get_rate() {
        let svc = test_service("zone-demo-1");
        let request = Request::new(pb::GetRateRequest {});
        let response = svc.get_rate(request).await;
        assert!(response.is_ok(), "GetRate should succeed when zone is configured");
        let resp = response.unwrap().into_inner();
        assert!(!resp.rate_type.is_empty(), "rate_type should not be empty");
        assert!(resp.rate_amount > 0.0, "rate_amount should be positive");
        assert!(!resp.currency.is_empty(), "currency should not be empty");
        assert_eq!(resp.zone_id, "zone-demo-1");
    }

    /// TS-08-E3: StartSession when session already active returns ALREADY_EXISTS.
    #[tokio::test]
    async fn test_start_session_already_active() {
        let session = Arc::new(Mutex::new(SessionManager::new()));
        // Manually set session to active state
        {
            let mut s = session.lock().await;
            s.try_start("zone-demo-1").unwrap();
            s.confirm_start("sess-123");
        }
        let svc = ParkingAdaptorService::new(session, "zone-demo-1".to_string());

        let request = Request::new(pb::StartSessionRequest {
            zone_id: "zone-demo-1".to_string(),
        });
        let response = svc.start_session(request).await;
        assert!(response.is_err(), "StartSession should fail when session is active");
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::AlreadyExists);
    }

    /// TS-08-E4: StopSession when no session active returns NOT_FOUND.
    #[tokio::test]
    async fn test_stop_session_no_active() {
        let svc = test_service("zone-demo-1");
        let request = Request::new(pb::StopSessionRequest {});
        let response = svc.stop_session(request).await;
        assert!(response.is_err(), "StopSession should fail when no session is active");
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    /// TS-08-E6: GetRate with no zone configured returns FAILED_PRECONDITION.
    #[tokio::test]
    async fn test_get_rate_no_zone() {
        let svc = test_service("");
        let request = Request::new(pb::GetRateRequest {});
        let response = svc.get_rate(request).await;
        assert!(response.is_err(), "GetRate should fail when no zone configured");
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);
    }
}
