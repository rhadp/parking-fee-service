//! gRPC service implementation for the ParkingOperatorAdaptorService.
//!
//! Delegates all operations to the event loop via a channel for
//! sequential processing (08-REQ-9.1). The gRPC server runs in a
//! spawned task and communicates with the event loop through
//! [`SessionEvent`] messages sent over an mpsc channel.

use tokio::sync::{mpsc, oneshot};
use tonic::{Request, Response, Status};

use crate::event_loop::{EventError, SessionEvent};

/// Generated parking_adaptor.v1 gRPC types.
#[allow(dead_code)]
mod proto {
    tonic::include_proto!("parking_adaptor.v1");
}

use proto::{
    parking_operator_adaptor_service_server::ParkingOperatorAdaptorService as ServiceTrait,
    GetRateRequest, GetRateResponse, GetStatusRequest, GetStatusResponse, ParkingRate,
    SessionStatus, StartSessionRequest, StartSessionResponse, StopSessionRequest,
    StopSessionResponse,
};

/// Re-export the server wrapper for use in main.rs.
pub use proto::parking_operator_adaptor_service_server::ParkingOperatorAdaptorServiceServer;

/// gRPC service implementation for ParkingOperatorAdaptorService.
///
/// Holds a channel sender to delegate all operations to the event loop.
/// Each RPC sends a [`SessionEvent`] and awaits a reply via a oneshot channel.
pub struct ParkingAdaptorService {
    event_tx: mpsc::Sender<SessionEvent>,
}

impl ParkingAdaptorService {
    /// Create a new service instance that sends events to the given channel.
    pub fn new(event_tx: mpsc::Sender<SessionEvent>) -> Self {
        Self { event_tx }
    }
}

#[tonic::async_trait]
impl ServiceTrait for ParkingAdaptorService {
    /// Start a parking session (08-REQ-1.2).
    ///
    /// Returns ALREADY_EXISTS if a session is active (08-REQ-1.E1).
    /// Returns UNAVAILABLE if the operator backend is unreachable (08-REQ-2.E1).
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
            .map_err(|_| Status::internal("event loop shut down"))?;

        let result = reply_rx
            .await
            .map_err(|_| Status::internal("event loop dropped reply"))?;

        match result {
            Ok(r) => Ok(Response::new(StartSessionResponse {
                session: Some(SessionStatus {
                    session_id: r.session_id,
                    active: true,
                    start_time: r.start_time,
                    zone_id: r.zone_id,
                }),
            })),
            Err(EventError::AlreadyExists { session_id }) => Err(Status::already_exists(
                format!("session already exists: {session_id}"),
            )),
            Err(EventError::OperatorUnavailable(msg)) => Err(Status::unavailable(msg)),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    /// Stop the active parking session (08-REQ-1.3).
    ///
    /// Returns FAILED_PRECONDITION if no session is active (08-REQ-1.E2).
    /// Returns UNAVAILABLE if the operator backend is unreachable (08-REQ-2.E1).
    async fn stop_session(
        &self,
        _request: Request<StopSessionRequest>,
    ) -> Result<Response<StopSessionResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::ManualStop { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("event loop shut down"))?;

        let result = reply_rx
            .await
            .map_err(|_| Status::internal("event loop dropped reply"))?;

        match result {
            Ok(r) => Ok(Response::new(StopSessionResponse {
                session: Some(SessionStatus {
                    session_id: r.session_id,
                    active: false,
                    start_time: 0,
                    zone_id: String::new(),
                }),
            })),
            Err(EventError::NoActiveSession) => {
                Err(Status::failed_precondition("no active session"))
            }
            Err(EventError::OperatorUnavailable(msg)) => Err(Status::unavailable(msg)),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    /// Return the current session state (08-REQ-1.4).
    ///
    /// Returns active=false with empty fields when no session is active.
    async fn get_status(
        &self,
        _request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::QueryStatus { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("event loop shut down"))?;

        let status = reply_rx
            .await
            .map_err(|_| Status::internal("event loop dropped reply"))?;

        let session = match status {
            Some(s) => SessionStatus {
                session_id: s.session_id,
                active: s.active,
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

        Ok(Response::new(GetStatusResponse {
            session: Some(session),
        }))
    }

    /// Return the cached rate from the active session (08-REQ-1.5).
    ///
    /// Returns an empty rate response when no session is active.
    async fn get_rate(
        &self,
        _request: Request<GetRateRequest>,
    ) -> Result<Response<GetRateResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::QueryRate { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("event loop shut down"))?;

        let rate = reply_rx
            .await
            .map_err(|_| Status::internal("event loop dropped reply"))?;

        let parking_rate = match rate {
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

        Ok(Response::new(GetRateResponse {
            rate: Some(parking_rate),
        }))
    }
}
