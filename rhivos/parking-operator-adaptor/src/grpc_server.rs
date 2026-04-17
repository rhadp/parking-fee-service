//! gRPC server implementation for ParkingAdaptor service.
//!
//! Routes gRPC calls to the event loop via an mpsc channel, preserving
//! sequential processing of all session state mutations (08-REQ-9.1).

use tokio::sync::oneshot;
use tonic::{Request, Response, Status};

use crate::event_loop::EventError;
use crate::operator::{StartResponse, StopResponse};
use crate::session::{Rate, SessionState};

/// Generated types from `proto/parking_adaptor.proto`.
pub mod proto {
    tonic::include_proto!("parking.adaptor");
}

use proto::parking_adaptor_server::ParkingAdaptor;
use proto::{
    GetRateRequest, GetRateResponse, GetStatusRequest, GetStatusResponse, StartSessionRequest,
    StartSessionResponse, StopSessionRequest, StopSessionResponse,
};

// ── Session events ─────────────────────────────────────────────────────────────

/// Commands sent from the gRPC handlers to the event loop.
pub enum SessionCommand {
    /// Manual StartSession gRPC call.
    ManualStart {
        zone_id: String,
        reply: oneshot::Sender<Result<StartResponse, EventError>>,
    },
    /// Manual StopSession gRPC call.
    ManualStop {
        reply: oneshot::Sender<Result<StopResponse, EventError>>,
    },
    /// GetStatus query.
    QueryStatus {
        reply: oneshot::Sender<Option<SessionState>>,
    },
    /// GetRate query.
    QueryRate {
        reply: oneshot::Sender<Option<Rate>>,
    },
}

// ── gRPC service ──────────────────────────────────────────────────────────────

/// `ParkingAdaptorService` implements the tonic-generated `ParkingAdaptor` trait.
///
/// Each RPC handler sends a `SessionCommand` to the event loop and awaits the
/// reply on a oneshot channel, ensuring serialized access to session state.
pub struct ParkingAdaptorService {
    tx: tokio::sync::mpsc::Sender<SessionCommand>,
}

impl ParkingAdaptorService {
    pub fn new(tx: tokio::sync::mpsc::Sender<SessionCommand>) -> Self {
        Self { tx }
    }
}

#[tonic::async_trait]
impl ParkingAdaptor for ParkingAdaptorService {
    // TS-08-2, TS-08-E1: StartSession
    async fn start_session(
        &self,
        request: Request<StartSessionRequest>,
    ) -> Result<Response<StartSessionResponse>, Status> {
        let zone_id = request.into_inner().zone_id;
        let (reply_tx, reply_rx) = oneshot::channel();

        self.tx
            .send(SessionCommand::ManualStart { zone_id, reply: reply_tx })
            .await
            .map_err(|_| Status::internal("Event loop unavailable"))?;

        match reply_rx.await {
            Ok(Ok(resp)) => {
                let rate = resp.rate;
                Ok(Response::new(StartSessionResponse {
                    session_id: resp.session_id,
                    status: resp.status,
                    rate: Some(proto::Rate {
                        rate_type: rate.rate_type,
                        amount: rate.amount,
                        currency: rate.currency,
                    }),
                }))
            }
            Ok(Err(EventError::AlreadyActive { existing_session_id })) => {
                Err(Status::already_exists(format!(
                    "Session already active: {existing_session_id}"
                )))
            }
            Ok(Err(EventError::OperatorFailed(msg))) => {
                Err(Status::unavailable(format!("Operator error: {msg}")))
            }
            Ok(Err(EventError::NotActive)) => {
                Err(Status::failed_precondition("No active session"))
            }
            Err(_) => Err(Status::internal("Event loop dropped the reply channel")),
        }
    }

    // TS-08-3, TS-08-E2: StopSession
    async fn stop_session(
        &self,
        _request: Request<StopSessionRequest>,
    ) -> Result<Response<StopSessionResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.tx
            .send(SessionCommand::ManualStop { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("Event loop unavailable"))?;

        match reply_rx.await {
            Ok(Ok(resp)) => Ok(Response::new(StopSessionResponse {
                session_id: resp.session_id,
                status: resp.status,
                duration_seconds: resp.duration_seconds,
                total_amount: resp.total_amount,
                currency: resp.currency,
            })),
            Ok(Err(EventError::NotActive)) => {
                Err(Status::failed_precondition("No active session to stop"))
            }
            Ok(Err(EventError::OperatorFailed(msg))) => {
                Err(Status::unavailable(format!("Operator error: {msg}")))
            }
            Ok(Err(EventError::AlreadyActive { .. })) => {
                Err(Status::internal("Unexpected AlreadyActive error in stop"))
            }
            Err(_) => Err(Status::internal("Event loop dropped the reply channel")),
        }
    }

    // TS-08-4, TS-08-5: GetStatus
    async fn get_status(
        &self,
        _request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.tx
            .send(SessionCommand::QueryStatus { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("Event loop unavailable"))?;

        let state = reply_rx
            .await
            .map_err(|_| Status::internal("Event loop dropped the reply channel"))?;

        let resp = match state {
            Some(s) => GetStatusResponse {
                active: true,
                session_id: s.session_id,
                zone_id: s.zone_id,
                start_time: s.start_time,
                rate: Some(proto::Rate {
                    rate_type: s.rate.rate_type,
                    amount: s.rate.amount,
                    currency: s.rate.currency,
                }),
            },
            None => GetStatusResponse {
                active: false,
                session_id: String::new(),
                zone_id: String::new(),
                start_time: 0,
                rate: None,
            },
        };

        Ok(Response::new(resp))
    }

    // TS-08-6, TS-08-7: GetRate
    async fn get_rate(
        &self,
        _request: Request<GetRateRequest>,
    ) -> Result<Response<GetRateResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.tx
            .send(SessionCommand::QueryRate { reply: reply_tx })
            .await
            .map_err(|_| Status::internal("Event loop unavailable"))?;

        let rate = reply_rx
            .await
            .map_err(|_| Status::internal("Event loop dropped the reply channel"))?;

        let resp = match rate {
            Some(r) => GetRateResponse {
                rate_type: r.rate_type,
                amount: r.amount,
                currency: r.currency,
            },
            None => GetRateResponse {
                rate_type: String::new(),
                amount: 0.0,
                currency: String::new(),
            },
        };

        Ok(Response::new(resp))
    }
}
