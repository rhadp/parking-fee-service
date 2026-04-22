use tokio::sync::{mpsc, oneshot};
use tonic::{Request, Response, Status};

use crate::broker::parking_adaptor::v1::parking_adaptor_server::ParkingAdaptor;
use crate::broker::parking_adaptor::v1::{
    GetRateRequest, GetStatusRequest, ParkingRate, SessionStatus, StartSessionRequest,
    StartSessionResponse, StopSessionRequest, StopSessionResponse,
};
use crate::event_loop::{ManualStartError, ManualStopError};
use crate::operator::{StartResponse, StopResponse};
use crate::session::{Rate, SessionState};

/// Internal event type for serialized processing through the event loop channel.
///
/// gRPC handlers and DATA_BROKER subscription send events through this enum.
/// A single task processes them sequentially (08-REQ-9.1).
pub enum SessionEvent {
    /// Lock state changed (from DATA_BROKER subscription).
    LockChanged(bool),
    /// Manual start session request (from gRPC StartSession).
    ManualStart {
        zone_id: String,
        reply: oneshot::Sender<Result<StartResponse, ManualStartError>>,
    },
    /// Manual stop session request (from gRPC StopSession).
    ManualStop {
        reply: oneshot::Sender<Result<StopResponse, ManualStopError>>,
    },
    /// Query session status (from gRPC GetStatus).
    QueryStatus {
        reply: oneshot::Sender<Option<SessionState>>,
    },
    /// Query session rate (from gRPC GetRate).
    QueryRate {
        reply: oneshot::Sender<Option<Rate>>,
    },
}

/// gRPC service implementation for the ParkingAdaptor service.
///
/// Sends events through a channel to the event loop for serialized processing.
/// Uses oneshot channels to receive responses back.
pub struct ParkingAdaptorService {
    event_tx: mpsc::Sender<SessionEvent>,
}

impl ParkingAdaptorService {
    /// Create a new ParkingAdaptorService with the given event channel sender.
    pub fn new(event_tx: mpsc::Sender<SessionEvent>) -> Self {
        Self { event_tx }
    }
}

#[tonic::async_trait]
impl ParkingAdaptor for ParkingAdaptorService {
    /// Start a parking session (08-REQ-1.2).
    ///
    /// Returns ALREADY_EXISTS if a session is already active (08-REQ-1.E1).
    async fn start_session(
        &self,
        request: Request<StartSessionRequest>,
    ) -> Result<Response<StartSessionResponse>, Status> {
        let req = request.into_inner();
        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::ManualStart {
                zone_id: req.zone_id,
                reply: reply_tx,
            })
            .await
            .map_err(|_| Status::internal("event loop closed"))?;

        let resp = reply_rx
            .await
            .map_err(|_| Status::internal("event loop dropped reply"))?
            .map_err(|e| match e {
                ManualStartError::AlreadyExists(id) => {
                    Status::already_exists(format!("session already active: {id}"))
                }
                ManualStartError::OperatorFailed(op_err) => {
                    Status::unavailable(format!("operator error: {op_err}"))
                }
            })?;

        Ok(Response::new(StartSessionResponse {
            session_id: resp.session_id,
            status: resp.status,
            rate: Some(ParkingRate {
                operator_id: String::new(),
                rate_type: resp.rate.rate_type,
                amount: resp.rate.amount,
                currency: resp.rate.currency,
            }),
        }))
    }

    /// Stop the active parking session (08-REQ-1.3).
    ///
    /// Returns FAILED_PRECONDITION if no session is active (08-REQ-1.E2).
    async fn stop_session(
        &self,
        _request: Request<StopSessionRequest>,
    ) -> Result<Response<StopSessionResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::ManualStop { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("event loop closed"))?;

        let resp = reply_rx
            .await
            .map_err(|_| Status::internal("event loop dropped reply"))?
            .map_err(|e| match e {
                ManualStopError::NoActiveSession => {
                    Status::failed_precondition("no active session")
                }
                ManualStopError::OperatorFailed(op_err) => {
                    Status::unavailable(format!("operator error: {op_err}"))
                }
            })?;

        Ok(Response::new(StopSessionResponse {
            session_id: resp.session_id,
            status: resp.status,
            duration_seconds: resp.duration_seconds,
            total_amount: resp.total_amount,
            currency: resp.currency,
        }))
    }

    /// Get the current session status (08-REQ-1.4).
    ///
    /// Returns active=false with empty fields when no session is active.
    async fn get_status(
        &self,
        _request: Request<GetStatusRequest>,
    ) -> Result<Response<SessionStatus>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::QueryStatus { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("event loop closed"))?;

        let state = reply_rx
            .await
            .map_err(|_| Status::internal("event loop dropped reply"))?;

        let response = match state {
            Some(s) => SessionStatus {
                session_id: s.session_id,
                active: true,
                start_time: s.start_time,
                zone_id: s.zone_id,
            },
            None => SessionStatus {
                session_id: String::new(),
                active: false,
                start_time: 0,
                zone_id: String::new(),
            },
        };

        Ok(Response::new(response))
    }

    /// Get the cached rate from the active session (08-REQ-1.5).
    ///
    /// Returns empty rate when no session is active.
    async fn get_rate(
        &self,
        _request: Request<GetRateRequest>,
    ) -> Result<Response<ParkingRate>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::QueryRate { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("event loop closed"))?;

        let rate = reply_rx
            .await
            .map_err(|_| Status::internal("event loop dropped reply"))?;

        let response = match rate {
            Some(r) => ParkingRate {
                operator_id: String::new(),
                rate_type: r.rate_type,
                amount: r.amount,
                currency: r.currency,
            },
            None => ParkingRate {
                operator_id: String::new(),
                rate_type: String::new(),
                amount: 0.0,
                currency: String::new(),
            },
        };

        Ok(Response::new(response))
    }
}
