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
