//! gRPC service implementation for `ParkingOperatorAdaptorService`.
//!
//! Handles `StartSession`, `StopSession`, `GetStatus`, and `GetRate`
//! RPCs by delegating to the event processing loop through a channel.

use crate::event_loop::{ManualError, SessionEvent};
use crate::proto::parking_adaptor::v1::{
    parking_operator_adaptor_service_server::ParkingOperatorAdaptorService, GetRateRequest,
    GetRateResponse, GetStatusRequest, GetStatusResponse, ParkingRate, SessionStatus,
    StartSessionRequest, StartSessionResponse, StopSessionRequest, StopSessionResponse,
};
use tokio::sync::{mpsc, oneshot};
use tonic::{Request, Response, Status};

/// gRPC service implementation for the parking operator adaptor.
///
/// Sends all session commands through the event channel to be processed
/// sequentially by the event loop. Uses oneshot channels for
/// request-response communication.
pub struct ParkingAdaptorServiceImpl {
    event_tx: mpsc::Sender<SessionEvent>,
}

impl ParkingAdaptorServiceImpl {
    /// Create a new gRPC service with the given event channel sender.
    pub fn new(event_tx: mpsc::Sender<SessionEvent>) -> Self {
        Self { event_tx }
    }
}

#[tonic::async_trait]
impl ParkingOperatorAdaptorService for ParkingAdaptorServiceImpl {
    async fn start_session(
        &self,
        request: Request<StartSessionRequest>,
    ) -> Result<Response<StartSessionResponse>, Status> {
        let req = request.into_inner();
        let zone_id = req.zone_id;

        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::ManualStart {
                zone_id: zone_id.clone(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| Status::internal("event loop not running"))?;

        let result = reply_rx
            .await
            .map_err(|_| Status::internal("event loop dropped reply"))?;

        match result {
            Ok(start_resp) => {
                // Query session state to get the authoritative start_time.
                let (status_tx, status_rx) = oneshot::channel();
                self.event_tx
                    .send(SessionEvent::QueryStatus { reply: status_tx })
                    .await
                    .map_err(|_| Status::internal("event loop not running"))?;
                let status = status_rx
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
                        session_id: start_resp.session_id,
                        active: true,
                        start_time: 0,
                        zone_id,
                    },
                };

                Ok(Response::new(StartSessionResponse {
                    session: Some(session),
                }))
            }
            Err(ManualError::AlreadyExists(id)) => {
                Err(Status::already_exists(format!("session already active: {id}")))
            }
            Err(ManualError::FailedPrecondition) => {
                Err(Status::failed_precondition("no active session"))
            }
            Err(ManualError::OperatorUnavailable(msg)) => Err(Status::unavailable(msg)),
        }
    }

    async fn stop_session(
        &self,
        _request: Request<StopSessionRequest>,
    ) -> Result<Response<StopSessionResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::ManualStop { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("event loop not running"))?;

        let result = reply_rx
            .await
            .map_err(|_| Status::internal("event loop dropped reply"))?;

        match result {
            Ok(stop_resp) => Ok(Response::new(StopSessionResponse {
                session: Some(SessionStatus {
                    session_id: stop_resp.session_id,
                    active: false,
                    start_time: 0,
                    zone_id: String::new(),
                }),
            })),
            Err(ManualError::AlreadyExists(id)) => {
                Err(Status::already_exists(format!("session already active: {id}")))
            }
            Err(ManualError::FailedPrecondition) => {
                Err(Status::failed_precondition("no active session"))
            }
            Err(ManualError::OperatorUnavailable(msg)) => Err(Status::unavailable(msg)),
        }
    }

    async fn get_status(
        &self,
        _request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::QueryStatus { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("event loop not running"))?;

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

    async fn get_rate(
        &self,
        _request: Request<GetRateRequest>,
    ) -> Result<Response<GetRateResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.event_tx
            .send(SessionEvent::QueryRate { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("event loop not running"))?;

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
