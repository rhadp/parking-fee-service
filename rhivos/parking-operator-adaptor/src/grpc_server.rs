use crate::event_loop::ProcessError;
use crate::operator::{StartResponse, StopResponse};
use crate::proto::parking_adaptor as pb;
use crate::session::{Rate, SessionState};
use tokio::sync::{mpsc, oneshot};
use tonic::{Request, Response, Status};

/// Internal event type for serialized processing through the event loop.
pub enum SessionEvent {
    /// Lock state changed (from DATA_BROKER subscription).
    LockChanged(bool),
    /// Manual StartSession request from gRPC.
    ManualStart {
        zone_id: String,
        reply: oneshot::Sender<Result<StartResponse, ProcessError>>,
    },
    /// Manual StopSession request from gRPC.
    ManualStop {
        reply: oneshot::Sender<Result<StopResponse, ProcessError>>,
    },
    /// Query current session status.
    QueryStatus {
        reply: oneshot::Sender<Option<SessionState>>,
    },
    /// Query current session rate.
    QueryRate {
        reply: oneshot::Sender<Option<Rate>>,
    },
}

/// gRPC service implementation for ParkingAdaptorService.
///
/// All state-mutating operations are sent to the event loop via the channel
/// to ensure serialized processing.
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
impl pb::parking_adaptor_service_server::ParkingAdaptorService for ParkingAdaptorService {
    async fn start_session(
        &self,
        request: Request<pb::StartSessionRequest>,
    ) -> Result<Response<pb::StartSessionResponse>, Status> {
        let zone_id = request.into_inner().zone_id;
        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::ManualStart {
                zone_id,
                reply: reply_tx,
            })
            .await
            .map_err(|_| Status::internal("event loop not available"))?;

        let result = reply_rx
            .await
            .map_err(|_| Status::internal("event loop dropped reply channel"))?;

        match result {
            Ok(resp) => Ok(Response::new(pb::StartSessionResponse {
                session_id: resp.session_id,
                status: resp.status,
                rate: Some(pb::Rate {
                    rate_type: resp.rate.rate_type,
                    amount: resp.rate.amount,
                    currency: resp.rate.currency,
                }),
            })),
            Err(ProcessError::AlreadyActive(id)) => Err(Status::already_exists(format!(
                "session already active: {id}"
            ))),
            Err(ProcessError::OperatorFailed(msg)) => {
                Err(Status::unavailable(format!("operator call failed: {msg}")))
            }
            Err(e) => Err(Status::internal(format!("unexpected error: {e}"))),
        }
    }

    async fn stop_session(
        &self,
        _request: Request<pb::StopSessionRequest>,
    ) -> Result<Response<pb::StopSessionResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::ManualStop { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("event loop not available"))?;

        let result = reply_rx
            .await
            .map_err(|_| Status::internal("event loop dropped reply channel"))?;

        match result {
            Ok(resp) => Ok(Response::new(pb::StopSessionResponse {
                session_id: resp.session_id,
                status: resp.status,
                duration_seconds: resp.duration_seconds,
                total_amount: resp.total_amount,
                currency: resp.currency,
            })),
            Err(ProcessError::NoActiveSession) => {
                Err(Status::failed_precondition("no active session"))
            }
            Err(ProcessError::OperatorFailed(msg)) => {
                Err(Status::unavailable(format!("operator call failed: {msg}")))
            }
            Err(e) => Err(Status::internal(format!("unexpected error: {e}"))),
        }
    }

    async fn get_status(
        &self,
        _request: Request<pb::GetStatusRequest>,
    ) -> Result<Response<pb::GetStatusResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::QueryStatus { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("event loop not available"))?;

        let state = reply_rx
            .await
            .map_err(|_| Status::internal("event loop dropped reply channel"))?;

        match state {
            Some(s) => Ok(Response::new(pb::GetStatusResponse {
                session_id: s.session_id,
                active: s.active,
                zone_id: s.zone_id,
                start_time: s.start_time,
                rate: Some(pb::Rate {
                    rate_type: s.rate.rate_type,
                    amount: s.rate.amount,
                    currency: s.rate.currency,
                }),
            })),
            None => Ok(Response::new(pb::GetStatusResponse {
                session_id: String::new(),
                active: false,
                zone_id: String::new(),
                start_time: 0,
                rate: None,
            })),
        }
    }

    async fn get_rate(
        &self,
        _request: Request<pb::GetRateRequest>,
    ) -> Result<Response<pb::GetRateResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::QueryRate { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("event loop not available"))?;

        let rate = reply_rx
            .await
            .map_err(|_| Status::internal("event loop dropped reply channel"))?;

        match rate {
            Some(r) => Ok(Response::new(pb::GetRateResponse {
                rate_type: r.rate_type,
                amount: r.amount,
                currency: r.currency,
            })),
            None => Ok(Response::new(pb::GetRateResponse {
                rate_type: String::new(),
                amount: 0.0,
                currency: String::new(),
            })),
        }
    }
}
