use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::operator::{OperatorClient, OperatorError};
use crate::session::{SessionError, SessionManager, SessionState};

/// Generated protobuf types from parking_adaptor.proto.
pub mod pb {
    tonic::include_proto!("parking_adaptor");
}

/// gRPC service implementation for the ParkingAdaptor.
pub struct ParkingAdaptorService {
    session: Arc<Mutex<SessionManager>>,
    operator: Arc<OperatorClient>,
    vehicle_id: String,
    zone_id: String,
}

impl ParkingAdaptorService {
    pub fn new(
        session: Arc<Mutex<SessionManager>>,
        operator: Arc<OperatorClient>,
        vehicle_id: String,
        zone_id: String,
    ) -> Self {
        Self {
            session,
            operator,
            vehicle_id,
            zone_id,
        }
    }

    /// Maps an OperatorError to a gRPC Status.
    fn map_operator_error(err: &OperatorError) -> Status {
        match err {
            OperatorError::Unreachable(msg) => {
                Status::unavailable(format!("operator unreachable: {msg}"))
            }
            OperatorError::Timeout => Status::deadline_exceeded("operator request timed out"),
            OperatorError::HttpError(code, body) => {
                Status::internal(format!("operator returned HTTP {code}: {body}"))
            }
            OperatorError::ParseError(msg) => {
                Status::internal(format!("failed to parse operator response: {msg}"))
            }
        }
    }
}

#[tonic::async_trait]
impl pb::parking_adaptor_server::ParkingAdaptor for ParkingAdaptorService {
    async fn start_session(
        &self,
        request: Request<pb::StartSessionRequest>,
    ) -> Result<Response<pb::StartSessionResponse>, Status> {
        let zone_id = request.into_inner().zone_id;
        let zone = if zone_id.is_empty() {
            self.zone_id.clone()
        } else {
            zone_id
        };

        // Acquire lock and attempt state transition
        let mut session = self.session.lock().await;
        session.try_start(&zone).map_err(|e| match e {
            SessionError::AlreadyActive => Status::already_exists("session already active"),
            _ => Status::internal("invalid state transition"),
        })?;

        // Call operator REST API
        match self
            .operator
            .start_session(&self.vehicle_id, &zone)
            .await
        {
            Ok(resp) => {
                session.confirm_start(&resp.session_id);
                // TODO: publish SessionActive = true to DATA_BROKER (task group 3)
                tracing::info!(session_id = %resp.session_id, "session started");
                Ok(Response::new(pb::StartSessionResponse {
                    session_id: resp.session_id,
                    status: resp.status,
                }))
            }
            Err(err) => {
                tracing::error!(?err, "operator start_session failed");
                session.fail_start();
                Err(Self::map_operator_error(&err))
            }
        }
    }

    async fn stop_session(
        &self,
        _request: Request<pb::StopSessionRequest>,
    ) -> Result<Response<pb::StopSessionResponse>, Status> {
        // Acquire lock and attempt state transition
        let mut session = self.session.lock().await;
        let session_id = session.try_stop().map_err(|e| match e {
            SessionError::NoActiveSession => Status::not_found("no active session"),
            _ => Status::internal("invalid state transition"),
        })?;

        // Call operator REST API
        match self.operator.stop_session(&session_id).await {
            Ok(resp) => {
                session.confirm_stop();
                // TODO: publish SessionActive = false to DATA_BROKER (task group 3)
                tracing::info!(session_id = %resp.session_id, "session stopped");
                Ok(Response::new(pb::StopSessionResponse {
                    session_id: resp.session_id,
                    duration_seconds: resp.duration,
                    fee: resp.fee,
                    status: resp.status,
                }))
            }
            Err(err) => {
                tracing::error!(?err, "operator stop_session failed");
                session.fail_stop();
                Err(Self::map_operator_error(&err))
            }
        }
    }

    async fn get_status(
        &self,
        _request: Request<pb::GetStatusRequest>,
    ) -> Result<Response<pb::GetStatusResponse>, Status> {
        let session = self.session.lock().await;
        let state = match session.state() {
            SessionState::Idle => "idle",
            SessionState::Starting => "starting",
            SessionState::Active => "active",
            SessionState::Stopping => "stopping",
        };
        let session_id = session.session_id().unwrap_or("").to_string();
        let zone_id = session.zone_id().unwrap_or("").to_string();

        Ok(Response::new(pb::GetStatusResponse {
            state: state.to_string(),
            session_id,
            zone_id,
        }))
    }

    async fn get_rate(
        &self,
        _request: Request<pb::GetRateRequest>,
    ) -> Result<Response<pb::GetRateResponse>, Status> {
        if self.zone_id.is_empty() {
            return Err(Status::failed_precondition("no zone configured"));
        }

        // Return default rate information for the configured zone
        // In a production system, this would query the operator
        Ok(Response::new(pb::GetRateResponse {
            rate_type: "per_hour".to_string(),
            rate_amount: 2.50,
            currency: "EUR".to_string(),
            zone_id: self.zone_id.clone(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pb::parking_adaptor_server::ParkingAdaptor;

    /// Helper to create a service instance for testing.
    fn test_service(zone_id: &str) -> ParkingAdaptorService {
        let session = Arc::new(Mutex::new(SessionManager::new()));
        let operator = Arc::new(OperatorClient::new("http://localhost:8080"));
        ParkingAdaptorService::new(
            session,
            operator,
            "DEMO-VIN-001".to_string(),
            zone_id.to_string(),
        )
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
        let operator = Arc::new(OperatorClient::new("http://localhost:8080"));
        let svc = ParkingAdaptorService::new(
            session,
            operator,
            "DEMO-VIN-001".to_string(),
            "zone-demo-1".to_string(),
        );

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
