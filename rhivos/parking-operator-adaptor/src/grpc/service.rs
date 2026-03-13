use crate::broker::SessionPublisher;
use crate::operator::client::OperatorError;
use crate::operator::OperatorApi;
use crate::session::{SessionManager, SessionState};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

/// The generated protobuf module for parking_adaptor.
pub mod proto {
    tonic::include_proto!("parking_adaptor");
}

/// gRPC service implementation for ParkingAdaptor.
pub struct ParkingAdaptorService {
    session: Arc<Mutex<SessionManager>>,
    operator: Arc<dyn OperatorApi>,
    vehicle_id: String,
    publisher: Arc<dyn SessionPublisher>,
}

impl ParkingAdaptorService {
    /// Create a new ParkingAdaptorService.
    pub fn new(
        session: Arc<Mutex<SessionManager>>,
        operator: Arc<dyn OperatorApi>,
        vehicle_id: String,
        publisher: Arc<dyn SessionPublisher>,
    ) -> Self {
        Self {
            session,
            operator,
            vehicle_id,
            publisher,
        }
    }
}

/// Map an OperatorError to a tonic::Status.
fn operator_error_to_status(err: &OperatorError) -> tonic::Status {
    match err {
        OperatorError::Unreachable(_) => {
            tonic::Status::unavailable(format!("operator unreachable: {err}"))
        }
        OperatorError::Timeout => {
            tonic::Status::deadline_exceeded("operator request timed out")
        }
        OperatorError::HttpError { status, body } => {
            tonic::Status::internal(format!("operator HTTP {status}: {body}"))
        }
        OperatorError::ParseError(msg) => {
            tonic::Status::internal(format!("operator response parse error: {msg}"))
        }
    }
}

#[tonic::async_trait]
impl proto::parking_adaptor_server::ParkingAdaptor for ParkingAdaptorService {
    async fn start_session(
        &self,
        request: tonic::Request<proto::StartSessionRequest>,
    ) -> Result<tonic::Response<proto::StartSessionResponse>, tonic::Status> {
        let zone_id = request.into_inner().zone_id;
        let mut session = self.session.lock().await;

        // Try to transition from Idle to Starting
        if let Err(_e) = session.try_start() {
            return Err(tonic::Status::already_exists("session already active"));
        }

        // Release lock before making the HTTP call
        drop(session);

        // Call operator REST API to start session
        match self.operator.start_session(&self.vehicle_id, &zone_id).await {
            Ok(resp) => {
                info!(session_id = %resp.session_id, "session started via gRPC");
                let mut session = self.session.lock().await;
                session.confirm_start(resp.session_id.clone());

                // Publish SessionActive = true to DATA_BROKER
                if let Err(e) = self.publisher.set_session_active(true).await {
                    error!(error = %e, "failed to publish SessionActive=true");
                }

                Ok(tonic::Response::new(proto::StartSessionResponse {
                    session_id: resp.session_id,
                    status: resp.status,
                }))
            }
            Err(e) => {
                error!(error = %e, "operator start_session failed");
                let mut session = self.session.lock().await;
                session.fail_start();
                Err(operator_error_to_status(&e))
            }
        }
    }

    async fn stop_session(
        &self,
        _request: tonic::Request<proto::StopSessionRequest>,
    ) -> Result<tonic::Response<proto::StopSessionResponse>, tonic::Status> {
        let mut session = self.session.lock().await;

        // Try to transition from Active to Stopping
        if let Err(_e) = session.try_stop() {
            return Err(tonic::Status::not_found("no active session"));
        }

        let session_id = session.session_id().unwrap_or_default().to_string();

        // Release lock before making the HTTP call
        drop(session);

        // Call operator REST API to stop session
        match self.operator.stop_session(&session_id).await {
            Ok(resp) => {
                info!(session_id = %resp.session_id, duration = resp.duration, fee = resp.fee, "session stopped via gRPC");
                let mut session = self.session.lock().await;
                session.confirm_stop();

                // Publish SessionActive = false to DATA_BROKER
                if let Err(e) = self.publisher.set_session_active(false).await {
                    error!(error = %e, "failed to publish SessionActive=false");
                }

                Ok(tonic::Response::new(proto::StopSessionResponse {
                    session_id: resp.session_id,
                    duration_seconds: resp.duration,
                    fee: resp.fee,
                    status: resp.status,
                }))
            }
            Err(e) => {
                error!(error = %e, "operator stop_session failed");
                let mut session = self.session.lock().await;
                session.fail_stop();
                Err(operator_error_to_status(&e))
            }
        }
    }

    async fn get_status(
        &self,
        _request: tonic::Request<proto::GetStatusRequest>,
    ) -> Result<tonic::Response<proto::GetStatusResponse>, tonic::Status> {
        let session = self.session.lock().await;
        let state_str = match session.state() {
            SessionState::Idle => "idle",
            SessionState::Starting => "starting",
            SessionState::Active => "active",
            SessionState::Stopping => "stopping",
        };
        Ok(tonic::Response::new(proto::GetStatusResponse {
            state: state_str.to_string(),
            session_id: session.session_id().unwrap_or_default().to_string(),
            zone_id: session.zone_id().unwrap_or_default().to_string(),
        }))
    }

    async fn get_rate(
        &self,
        _request: tonic::Request<proto::GetRateRequest>,
    ) -> Result<tonic::Response<proto::GetRateResponse>, tonic::Status> {
        let session = self.session.lock().await;
        let zone_id = session.zone_id();
        match zone_id {
            None | Some("") => {
                Err(tonic::Status::failed_precondition("no zone configured"))
            }
            Some(zone) => {
                Ok(tonic::Response::new(proto::GetRateResponse {
                    rate_type: "per_hour".to_string(),
                    rate_amount: 2.50,
                    currency: "EUR".to_string(),
                    zone_id: zone.to_string(),
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::proto::parking_adaptor_server::ParkingAdaptor;
    use super::*;
    use crate::testing::{MockBrokerPublisher, MockOperatorClient, NoopPublisher};

    fn make_service(zone_id: Option<String>) -> ParkingAdaptorService {
        let session = Arc::new(Mutex::new(SessionManager::new(zone_id)));
        let operator: Arc<dyn OperatorApi> = Arc::new(MockOperatorClient::new());
        let publisher: Arc<dyn SessionPublisher> = Arc::new(NoopPublisher);
        ParkingAdaptorService::new(session, operator, "DEMO-VIN-001".to_string(), publisher)
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

    /// TS-08-E5: StartSession returns ALREADY_EXISTS when session is active.
    #[tokio::test]
    async fn test_start_session_already_active() {
        let session = Arc::new(Mutex::new(SessionManager::new(Some("zone-1".to_string()))));
        // Simulate an active session
        {
            let mut s = session.lock().await;
            s.try_start().unwrap();
            s.confirm_start("session-123".to_string());
        }
        let operator: Arc<dyn OperatorApi> = Arc::new(MockOperatorClient::new());
        let publisher: Arc<dyn SessionPublisher> = Arc::new(NoopPublisher);
        let svc = ParkingAdaptorService::new(session, operator, "DEMO-VIN-001".to_string(), publisher);
        let request = tonic::Request::new(proto::StartSessionRequest {
            zone_id: "zone-1".to_string(),
        });
        let result = svc.start_session(request).await;
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(status.code(), tonic::Code::AlreadyExists);
    }

    /// TS-08-E6: StopSession returns NOT_FOUND when no session is active.
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

    /// TS-08-7: Manual StartSession via gRPC starts a session.
    #[tokio::test]
    async fn test_manual_start_session() {
        let session = Arc::new(Mutex::new(SessionManager::new(Some("zone-demo-1".to_string()))));
        let operator: Arc<dyn OperatorApi> = Arc::new(MockOperatorClient::new());
        let publisher: Arc<dyn SessionPublisher> = Arc::new(MockBrokerPublisher::new());
        let svc = ParkingAdaptorService::new(
            session.clone(),
            operator,
            "DEMO-VIN-001".to_string(),
            publisher,
        );

        let request = tonic::Request::new(proto::StartSessionRequest {
            zone_id: "zone-demo-1".to_string(),
        });
        let resp = svc.start_session(request).await.unwrap().into_inner();
        assert!(!resp.session_id.is_empty());
        assert_eq!(resp.status, "active");

        let s = session.lock().await;
        assert!(s.is_active());
    }

    /// TS-08-8: Manual StopSession via gRPC stops a session.
    #[tokio::test]
    async fn test_manual_stop_session() {
        let session = Arc::new(Mutex::new(SessionManager::new(Some("zone-demo-1".to_string()))));
        let operator: Arc<dyn OperatorApi> = Arc::new(MockOperatorClient::new());
        let publisher: Arc<dyn SessionPublisher> = Arc::new(MockBrokerPublisher::new());
        let svc = ParkingAdaptorService::new(
            session.clone(),
            operator,
            "DEMO-VIN-001".to_string(),
            publisher,
        );

        // Start first
        let start_req = tonic::Request::new(proto::StartSessionRequest {
            zone_id: "zone-demo-1".to_string(),
        });
        svc.start_session(start_req).await.unwrap();

        // Stop
        let stop_req = tonic::Request::new(proto::StopSessionRequest {});
        let resp = svc.stop_session(stop_req).await.unwrap().into_inner();
        assert!(!resp.session_id.is_empty());
        assert_eq!(resp.status, "completed");

        let s = session.lock().await;
        assert!(!s.is_active());
    }
}
