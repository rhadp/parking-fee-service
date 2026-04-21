//! gRPC service implementation for PARKING_OPERATOR_ADAPTOR.
//!
//! Implements the `ParkingAdaptor` service from `parking_adaptor.proto`.
//! All RPCs delegate to [`crate::event_loop`] functions via shared session state.

use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

use crate::broker::SessionPublisher;
use crate::event_loop::{manual_start, manual_stop, StartError, StopError};
use crate::operator::OperatorApi;
use crate::session::Session;

// ── Proto module ──────────────────────────────────────────────────────────────

pub mod proto {
    pub mod parking_adaptor {
        tonic::include_proto!("parking.adaptor");
    }
}

use proto::parking_adaptor::{
    parking_adaptor_server::ParkingAdaptor, GetRateRequest, GetRateResponse, GetStatusRequest,
    GetStatusResponse, RateInfo, StartSessionRequest, StartSessionResponse, StopSessionRequest,
    StopSessionResponse,
};

// ── ParkingAdaptorService ─────────────────────────────────────────────────────

/// gRPC service implementation.
///
/// Shares session state with the lock-event processing task via a Mutex.
/// Calls to manual_start / manual_stop / GetStatus / GetRate are serialised
/// by the Mutex, satisfying 08-REQ-9.1.
#[derive(Clone)]
pub struct ParkingAdaptorService {
    session: Arc<Mutex<Session>>,
    operator: Arc<dyn OperatorApi>,
    publisher: Arc<dyn SessionPublisher>,
    vehicle_id: String,
    default_zone_id: String,
}

impl ParkingAdaptorService {
    /// Create a new service instance.
    pub fn new(
        session: Arc<Mutex<Session>>,
        operator: Arc<dyn OperatorApi>,
        publisher: Arc<dyn SessionPublisher>,
        vehicle_id: String,
        default_zone_id: String,
    ) -> Self {
        Self {
            session,
            operator,
            publisher,
            vehicle_id,
            default_zone_id,
        }
    }
}

// ── ParkingAdaptor trait implementation ───────────────────────────────────────

pub use proto::parking_adaptor::parking_adaptor_server::ParkingAdaptorServer;

#[tonic::async_trait]
impl ParkingAdaptor for ParkingAdaptorService {
    /// Start a parking session (08-REQ-1.2, 08-REQ-1.E1, 08-REQ-5.1).
    async fn start_session(
        &self,
        request: Request<StartSessionRequest>,
    ) -> Result<Response<StartSessionResponse>, Status> {
        let zone_id = {
            let z = request.into_inner().zone_id;
            if z.is_empty() {
                self.default_zone_id.clone()
            } else {
                z
            }
        };

        let mut session = self.session.lock().await;

        match manual_start(
            &zone_id,
            &mut session,
            self.operator.as_ref(),
            self.publisher.as_ref(),
            &self.vehicle_id,
        )
        .await
        {
            Ok(resp) => Ok(Response::new(StartSessionResponse {
                session_id: resp.session_id,
                status: resp.status,
                rate: Some(RateInfo {
                    rate_type: resp.rate.rate_type,
                    amount: resp.rate.amount,
                    currency: resp.rate.currency,
                }),
            })),
            Err(StartError::AlreadyActive { session_id }) => Err(Status::already_exists(format!(
                "session {session_id} is already active"
            ))),
            Err(StartError::Operator(e)) => Err(Status::unavailable(e.to_string())),
        }
    }

    /// Stop the active parking session (08-REQ-1.3, 08-REQ-1.E2, 08-REQ-5.2).
    async fn stop_session(
        &self,
        _request: Request<StopSessionRequest>,
    ) -> Result<Response<StopSessionResponse>, Status> {
        let mut session = self.session.lock().await;

        match manual_stop(&mut session, self.operator.as_ref(), self.publisher.as_ref()).await {
            Ok(resp) => Ok(Response::new(StopSessionResponse {
                session_id: resp.session_id,
                status: resp.status,
                duration_seconds: resp.duration_seconds,
                total_amount: resp.total_amount,
                currency: resp.currency,
            })),
            Err(StopError::NotActive) => {
                Err(Status::failed_precondition("no parking session is active"))
            }
            Err(StopError::Operator(e)) => Err(Status::unavailable(e.to_string())),
        }
    }

    /// Return the current session status (08-REQ-1.4).
    async fn get_status(
        &self,
        _request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        let session = self.session.lock().await;

        match session.status() {
            Some(state) => Ok(Response::new(GetStatusResponse {
                active: true,
                session_id: state.session_id.clone(),
                zone_id: state.zone_id.clone(),
                start_time: state.start_time,
                rate: Some(RateInfo {
                    rate_type: state.rate.rate_type.clone(),
                    amount: state.rate.amount,
                    currency: state.rate.currency.clone(),
                }),
            })),
            None => Ok(Response::new(GetStatusResponse {
                active: false,
                session_id: String::new(),
                zone_id: String::new(),
                start_time: 0,
                rate: None,
            })),
        }
    }

    /// Return the cached rate from the active session (08-REQ-1.5).
    async fn get_rate(
        &self,
        _request: Request<GetRateRequest>,
    ) -> Result<Response<GetRateResponse>, Status> {
        let session = self.session.lock().await;

        match session.rate() {
            Some(rate) => Ok(Response::new(GetRateResponse {
                rate_type: rate.rate_type.clone(),
                amount: rate.amount,
                currency: rate.currency.clone(),
            })),
            None => Ok(Response::new(GetRateResponse {
                rate_type: String::new(),
                amount: 0.0,
                currency: String::new(),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::{BrokerError, SessionPublisher};
    use crate::operator::{OperatorApi, OperatorError, StartResponse, StopResponse};
    use crate::session::{Rate, Session};
    use async_trait::async_trait;
    use tonic::Code;

    // ── Test helpers ─────────────────────────────────────────────────────────

    fn make_rate() -> Rate {
        Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.50,
            currency: "EUR".to_string(),
        }
    }

    fn make_start_response() -> StartResponse {
        StartResponse {
            session_id: "sess-1".to_string(),
            status: "active".to_string(),
            rate: make_rate(),
        }
    }

    fn make_stop_response() -> StopResponse {
        StopResponse {
            session_id: "sess-1".to_string(),
            status: "completed".to_string(),
            duration_seconds: 3600,
            total_amount: 2.50,
            currency: "EUR".to_string(),
        }
    }

    // ── Mock operator ────────────────────────────────────────────────────────

    #[derive(Default)]
    struct MockOperator {
        start_response: Option<Result<StartResponse, OperatorError>>,
        stop_response: Option<Result<StopResponse, OperatorError>>,
    }

    #[async_trait]
    impl OperatorApi for MockOperator {
        async fn start_session(
            &self,
            _vehicle_id: &str,
            _zone_id: &str,
        ) -> Result<StartResponse, OperatorError> {
            self.start_response
                .clone()
                .unwrap_or_else(|| Ok(make_start_response()))
        }

        async fn stop_session(
            &self,
            _session_id: &str,
        ) -> Result<StopResponse, OperatorError> {
            self.stop_response
                .clone()
                .unwrap_or_else(|| Ok(make_stop_response()))
        }
    }

    // ── Mock publisher ───────────────────────────────────────────────────────

    #[derive(Default)]
    struct MockPublisher;

    #[async_trait]
    impl SessionPublisher for MockPublisher {
        async fn set_session_active(&self, _active: bool) -> Result<(), BrokerError> {
            Ok(())
        }
    }

    // ── Helper to build ParkingAdaptorService with mocks ─────────────────────

    fn build_service(
        session: Session,
        operator: MockOperator,
    ) -> ParkingAdaptorService {
        ParkingAdaptorService::new(
            Arc::new(Mutex::new(session)),
            Arc::new(operator),
            Arc::new(MockPublisher),
            "DEMO-VIN-001".to_string(),
            "zone-demo-1".to_string(),
        )
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    /// TS-08-E1 (gRPC layer): StartSession returns ALREADY_EXISTS gRPC error
    /// when a session is already active.
    ///
    /// Verifies: 08-REQ-1.E1 — the gRPC status code mapping from
    /// StartError::AlreadyActive to tonic::Code::AlreadyExists.
    #[tokio::test]
    async fn test_grpc_start_session_already_exists() {
        let mut session = Session::new();
        session.start("sess-1".into(), "zone-a".into(), 1_700_000_000, make_rate());

        let svc = build_service(session, MockOperator::default());
        let request = Request::new(StartSessionRequest {
            zone_id: "zone-b".to_string(),
        });

        let result = ParkingAdaptor::start_session(&svc, request).await;

        assert!(result.is_err(), "expected gRPC error");
        let status = result.unwrap_err();
        assert_eq!(
            status.code(),
            Code::AlreadyExists,
            "expected ALREADY_EXISTS gRPC code, got {:?}",
            status.code()
        );
    }

    /// TS-08-E2 (gRPC layer): StopSession returns FAILED_PRECONDITION gRPC
    /// error when no session is active.
    ///
    /// Verifies: 08-REQ-1.E2 — the gRPC status code mapping from
    /// StopError::NotActive to tonic::Code::FailedPrecondition.
    #[tokio::test]
    async fn test_grpc_stop_session_failed_precondition() {
        let session = Session::new();

        let svc = build_service(session, MockOperator::default());
        let request = Request::new(StopSessionRequest {});

        let result = ParkingAdaptor::stop_session(&svc, request).await;

        assert!(result.is_err(), "expected gRPC error");
        let status = result.unwrap_err();
        assert_eq!(
            status.code(),
            Code::FailedPrecondition,
            "expected FAILED_PRECONDITION gRPC code, got {:?}",
            status.code()
        );
    }

    /// TS-08-2 (gRPC layer): StartSession returns session_id, status, and rate.
    ///
    /// Verifies: 08-REQ-1.2 — the full gRPC response including currency field.
    #[tokio::test]
    async fn test_grpc_start_session_success() {
        let session = Session::new();

        let svc = build_service(session, MockOperator::default());
        let request = Request::new(StartSessionRequest {
            zone_id: "zone-a".to_string(),
        });

        let result = ParkingAdaptor::start_session(&svc, request).await;

        assert!(result.is_ok(), "expected successful response");
        let resp = result.unwrap().into_inner();
        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.status, "active");
        let rate = resp.rate.expect("rate must be present");
        assert_eq!(rate.rate_type, "per_hour");
        assert!((rate.amount - 2.50).abs() < f64::EPSILON);
        assert_eq!(rate.currency, "EUR");
    }

    /// TS-08-3 (gRPC layer): StopSession returns duration and total_amount.
    ///
    /// Verifies: 08-REQ-1.3 — the full gRPC stop response.
    #[tokio::test]
    async fn test_grpc_stop_session_success() {
        let mut session = Session::new();
        session.start("sess-1".into(), "zone-a".into(), 1_700_000_000, make_rate());

        let svc = build_service(session, MockOperator::default());
        let request = Request::new(StopSessionRequest {});

        let result = ParkingAdaptor::stop_session(&svc, request).await;

        assert!(result.is_ok(), "expected successful response");
        let resp = result.unwrap().into_inner();
        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.status, "completed");
        assert_eq!(resp.duration_seconds, 3600);
        assert!((resp.total_amount - 2.50).abs() < f64::EPSILON);
        assert_eq!(resp.currency, "EUR");
    }

    /// TS-08-2 (gRPC layer): StartSession returns UNAVAILABLE when the
    /// operator REST call fails.
    ///
    /// Verifies: 08-REQ-2.E1 — operator failure maps to UNAVAILABLE gRPC code.
    #[tokio::test]
    async fn test_grpc_start_session_operator_failure() {
        let session = Session::new();
        let operator = MockOperator {
            start_response: Some(Err(OperatorError::Unavailable("timeout".into()))),
            ..Default::default()
        };

        let svc = build_service(session, operator);
        let request = Request::new(StartSessionRequest {
            zone_id: "zone-a".to_string(),
        });

        let result = ParkingAdaptor::start_session(&svc, request).await;

        assert!(result.is_err(), "expected gRPC error");
        let status = result.unwrap_err();
        assert_eq!(
            status.code(),
            Code::Unavailable,
            "expected UNAVAILABLE gRPC code, got {:?}",
            status.code()
        );
    }

    /// StopSession returns UNAVAILABLE when the operator REST call fails.
    ///
    /// Verifies: 08-REQ-2.E1 — operator failure maps to UNAVAILABLE gRPC code.
    #[tokio::test]
    async fn test_grpc_stop_session_operator_failure() {
        let mut session = Session::new();
        session.start("sess-1".into(), "zone-a".into(), 1_700_000_000, make_rate());
        let operator = MockOperator {
            stop_response: Some(Err(OperatorError::Unavailable("timeout".into()))),
            ..Default::default()
        };

        let svc = build_service(session, operator);
        let request = Request::new(StopSessionRequest {});

        let result = ParkingAdaptor::stop_session(&svc, request).await;

        assert!(result.is_err(), "expected gRPC error");
        let status = result.unwrap_err();
        assert_eq!(
            status.code(),
            Code::Unavailable,
            "expected UNAVAILABLE gRPC code, got {:?}",
            status.code()
        );
    }

    /// GetStatus via gRPC returns active session details.
    ///
    /// Verifies: 08-REQ-1.4 — gRPC GetStatus response fields.
    #[tokio::test]
    async fn test_grpc_get_status_active() {
        let mut session = Session::new();
        session.start("sess-1".into(), "zone-a".into(), 1_700_000_000, make_rate());

        let svc = build_service(session, MockOperator::default());
        let result = ParkingAdaptor::get_status(&svc, Request::new(GetStatusRequest {})).await;

        assert!(result.is_ok());
        let resp = result.unwrap().into_inner();
        assert!(resp.active);
        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.zone_id, "zone-a");
        assert_eq!(resp.start_time, 1_700_000_000);
        let rate = resp.rate.expect("rate must be present");
        assert_eq!(rate.rate_type, "per_hour");
    }

    /// GetStatus via gRPC returns inactive state when no session.
    ///
    /// Verifies: 08-REQ-1.4 — gRPC GetStatus inactive response.
    #[tokio::test]
    async fn test_grpc_get_status_inactive() {
        let session = Session::new();
        let svc = build_service(session, MockOperator::default());
        let result = ParkingAdaptor::get_status(&svc, Request::new(GetStatusRequest {})).await;

        assert!(result.is_ok());
        let resp = result.unwrap().into_inner();
        assert!(!resp.active);
        assert_eq!(resp.session_id, "");
    }

    /// GetRate via gRPC returns cached rate from active session.
    ///
    /// Verifies: 08-REQ-1.5 — gRPC GetRate response fields.
    #[tokio::test]
    async fn test_grpc_get_rate_active() {
        let mut session = Session::new();
        session.start("sess-1".into(), "zone-a".into(), 0, make_rate());

        let svc = build_service(session, MockOperator::default());
        let result = ParkingAdaptor::get_rate(&svc, Request::new(GetRateRequest {})).await;

        assert!(result.is_ok());
        let resp = result.unwrap().into_inner();
        assert_eq!(resp.rate_type, "per_hour");
        assert!((resp.amount - 2.50).abs() < f64::EPSILON);
        assert_eq!(resp.currency, "EUR");
    }

    /// GetRate via gRPC returns empty rate when no session.
    ///
    /// Verifies: 08-REQ-1.5 — gRPC GetRate empty response.
    #[tokio::test]
    async fn test_grpc_get_rate_inactive() {
        let session = Session::new();
        let svc = build_service(session, MockOperator::default());
        let result = ParkingAdaptor::get_rate(&svc, Request::new(GetRateRequest {})).await;

        assert!(result.is_ok());
        let resp = result.unwrap().into_inner();
        assert_eq!(resp.rate_type, "");
        assert!((resp.amount - 0.0).abs() < f64::EPSILON);
        assert_eq!(resp.currency, "");
    }
}
