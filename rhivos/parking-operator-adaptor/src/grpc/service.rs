use crate::session::SessionManager;
use std::sync::Arc;
use tokio::sync::Mutex;

/// The generated protobuf module for parking_adaptor.
pub mod proto {
    tonic::include_proto!("parking_adaptor");
}

/// gRPC service implementation for ParkingAdaptor.
pub struct ParkingAdaptorService {
    _session: Arc<Mutex<SessionManager>>,
}

impl ParkingAdaptorService {
    /// Create a new ParkingAdaptorService.
    pub fn new(session: Arc<Mutex<SessionManager>>) -> Self {
        Self { _session: session }
    }
}

#[tonic::async_trait]
impl proto::parking_adaptor_server::ParkingAdaptor for ParkingAdaptorService {
    async fn start_session(
        &self,
        _request: tonic::Request<proto::StartSessionRequest>,
    ) -> Result<tonic::Response<proto::StartSessionResponse>, tonic::Status> {
        // Stub: will be implemented in task group 2
        todo!("StartSession not yet implemented")
    }

    async fn stop_session(
        &self,
        _request: tonic::Request<proto::StopSessionRequest>,
    ) -> Result<tonic::Response<proto::StopSessionResponse>, tonic::Status> {
        // Stub: will be implemented in task group 2
        todo!("StopSession not yet implemented")
    }

    async fn get_status(
        &self,
        _request: tonic::Request<proto::GetStatusRequest>,
    ) -> Result<tonic::Response<proto::GetStatusResponse>, tonic::Status> {
        // Stub: will be implemented in task group 2
        todo!("GetStatus not yet implemented")
    }

    async fn get_rate(
        &self,
        _request: tonic::Request<proto::GetRateRequest>,
    ) -> Result<tonic::Response<proto::GetRateResponse>, tonic::Status> {
        // Stub: will be implemented in task group 2
        todo!("GetRate not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::proto::parking_adaptor_server::ParkingAdaptor;
    use super::*;

    fn make_service(zone_id: Option<String>) -> ParkingAdaptorService {
        let session = Arc::new(Mutex::new(SessionManager::new(zone_id)));
        ParkingAdaptorService::new(session)
    }

    /// TS-08-5: GetStatus returns idle state when no session is active.
    #[tokio::test]
    async fn test_get_status_idle() {
        let svc = make_service(Some("zone-demo-1".to_string()));
        let request = tonic::Request::new(proto::GetStatusRequest {});
        let response = svc.get_status(request).await.unwrap();
        let resp = response.into_inner();
        assert_eq!(resp.state, "idle");
        assert!(resp.session_id.is_empty());
    }

    /// TS-08-6: GetRate returns rate information for configured zone.
    #[tokio::test]
    async fn test_get_rate() {
        let svc = make_service(Some("zone-demo-1".to_string()));
        let request = tonic::Request::new(proto::GetRateRequest {});
        let response = svc.get_rate(request).await.unwrap();
        let resp = response.into_inner();
        assert!(
            resp.rate_type == "per_hour" || resp.rate_type == "flat_fee",
            "rate_type must be per_hour or flat_fee"
        );
        assert!(resp.rate_amount > 0.0, "rate_amount must be positive");
        assert!(!resp.currency.is_empty(), "currency must not be empty");
        assert_eq!(resp.zone_id, "zone-demo-1");
    }

    /// TS-08-E3: StartSession returns ALREADY_EXISTS when session is active.
    #[tokio::test]
    async fn test_start_session_already_active() {
        let session = Arc::new(Mutex::new(SessionManager::new(Some("zone-1".to_string()))));
        // Simulate an active session by transitioning through the state machine
        {
            let mut s = session.lock().await;
            s.try_start().unwrap();
            s.confirm_start("session-123".to_string());
        }
        let svc = ParkingAdaptorService::new(session);
        let request = tonic::Request::new(proto::StartSessionRequest {
            zone_id: "zone-1".to_string(),
        });
        let result = svc.start_session(request).await;
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::AlreadyExists);
    }

    /// TS-08-E4: StopSession returns NOT_FOUND when no session is active.
    #[tokio::test]
    async fn test_stop_session_no_active() {
        let svc = make_service(Some("zone-demo-1".to_string()));
        let request = tonic::Request::new(proto::StopSessionRequest {});
        let result = svc.stop_session(request).await;
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    /// TS-08-E6: GetRate returns FAILED_PRECONDITION when no zone configured.
    #[tokio::test]
    async fn test_get_rate_no_zone() {
        let svc = make_service(None);
        let request = tonic::Request::new(proto::GetRateRequest {});
        let result = svc.get_rate(request).await;
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);
    }
}
