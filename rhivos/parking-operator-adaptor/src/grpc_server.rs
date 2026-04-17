//! gRPC service implementation for the ParkingAdaptor service.
//!
//! `ParkingService` dispatches all RPCs to the event loop via mpsc+oneshot
//! channels, ensuring session state mutations are serialised (08-REQ-9.1).

#![allow(clippy::result_large_err)]

use crate::event_loop::SessionCommand;
use crate::proto::parking as pb;
use tokio::sync::{mpsc, oneshot};
use tonic::{Request, Response, Status};

use pb::parking_adaptor_server::ParkingAdaptor;

/// gRPC service that dispatches all requests to the sequential event loop.
pub struct ParkingService {
    tx: mpsc::Sender<SessionCommand>,
}

impl ParkingService {
    /// Create a new service backed by the given event-loop command sender.
    pub fn new(tx: mpsc::Sender<SessionCommand>) -> Self {
        ParkingService { tx }
    }

    /// Send a command and await its oneshot reply.
    async fn send<T>(&self, cmd: SessionCommand, rx: oneshot::Receiver<T>) -> Result<T, Status> {
        self.tx
            .send(cmd)
            .await
            .map_err(|_| Status::internal("event loop unavailable"))?;
        rx.await
            .map_err(|_| Status::internal("event loop dropped reply channel"))
    }
}

#[tonic::async_trait]
impl ParkingAdaptor for ParkingService {
    /// StartSession RPC — delegates to `process_manual_start` via event loop.
    ///
    /// Returns ALREADY_EXISTS if a session is already active (08-REQ-1.E1).
    async fn start_session(
        &self,
        request: Request<pb::StartSessionRequest>,
    ) -> Result<Response<pb::StartSessionResponse>, Status> {
        let zone_id = request.into_inner().zone_id;
        let (reply_tx, reply_rx) = oneshot::channel();

        let result = self
            .send(
                SessionCommand::ManualStart {
                    zone_id,
                    reply: reply_tx,
                },
                reply_rx,
            )
            .await??; // double ? — outer unwraps send/recv error; inner unwraps domain error

        Ok(Response::new(pb::StartSessionResponse {
            session_id: result.session_id,
            status: result.status,
            rate: Some(pb::Rate {
                rate_type: result.rate.rate_type,
                amount: result.rate.amount,
                currency: result.rate.currency,
            }),
        }))
    }

    /// StopSession RPC — delegates to `process_manual_stop` via event loop.
    ///
    /// Returns FAILED_PRECONDITION if no session is active (08-REQ-1.E2).
    async fn stop_session(
        &self,
        _request: Request<pb::StopSessionRequest>,
    ) -> Result<Response<pb::StopSessionResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        let result = self
            .send(SessionCommand::ManualStop { reply: reply_tx }, reply_rx)
            .await??;

        Ok(Response::new(pb::StopSessionResponse {
            session_id: result.session_id,
            status: result.status,
            duration_seconds: result.duration_seconds as i64,
            total_amount: result.total_amount,
            currency: result.currency,
        }))
    }

    /// GetStatus RPC — reads in-memory session state (08-REQ-1.4).
    async fn get_status(
        &self,
        _request: Request<pb::GetStatusRequest>,
    ) -> Result<Response<pb::GetStatusResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        let state = self
            .send(SessionCommand::QueryStatus { reply: reply_tx }, reply_rx)
            .await?;

        let resp = match state {
            Some(s) => pb::GetStatusResponse {
                active: true,
                session_id: s.session_id,
                zone_id: s.zone_id,
                start_time: s.start_time,
                rate: Some(pb::Rate {
                    rate_type: s.rate.rate_type,
                    amount: s.rate.amount,
                    currency: s.rate.currency,
                }),
            },
            None => pb::GetStatusResponse {
                active: false,
                session_id: String::new(),
                zone_id: String::new(),
                start_time: 0,
                rate: None,
            },
        };

        Ok(Response::new(resp))
    }

    /// GetRate RPC — reads cached rate from in-memory session state (08-REQ-1.5).
    async fn get_rate(
        &self,
        _request: Request<pb::GetRateRequest>,
    ) -> Result<Response<pb::GetRateResponse>, Status> {
        let (reply_tx, reply_rx) = oneshot::channel();

        let rate = self
            .send(SessionCommand::QueryRate { reply: reply_tx }, reply_rx)
            .await?;

        let resp = match rate {
            Some(r) => pb::GetRateResponse {
                rate_type: r.rate_type,
                amount: r.amount,
                currency: r.currency,
            },
            None => pb::GetRateResponse {
                rate_type: String::new(),
                amount: 0.0,
                currency: String::new(),
            },
        };

        Ok(Response::new(resp))
    }
}
