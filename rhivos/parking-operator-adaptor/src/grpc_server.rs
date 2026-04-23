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
                rate: Some(ParkingRate {
                    operator_id: String::new(),
                    rate_type: s.rate.rate_type,
                    amount: s.rate.amount,
                    currency: s.rate.currency,
                }),
            },
            None => SessionStatus {
                session_id: String::new(),
                active: false,
                start_time: 0,
                zone_id: String::new(),
                rate: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::parking_adaptor::v1::parking_adaptor_server::ParkingAdaptor;
    use crate::operator::{OperatorError, RateResponse};

    /// Spawn a one-shot event handler that processes one ManualStart event
    /// and replies with the given result.
    async fn handle_manual_start(
        mut rx: mpsc::Receiver<SessionEvent>,
        result: Result<StartResponse, ManualStartError>,
    ) {
        if let Some(SessionEvent::ManualStart { reply, .. }) = rx.recv().await {
            let _ = reply.send(result);
        }
    }

    /// Spawn a one-shot event handler that processes one ManualStop event
    /// and replies with the given result.
    async fn handle_manual_stop(
        mut rx: mpsc::Receiver<SessionEvent>,
        result: Result<StopResponse, ManualStopError>,
    ) {
        if let Some(SessionEvent::ManualStop { reply, .. }) = rx.recv().await {
            let _ = reply.send(result);
        }
    }

    // TS-08-E1 (gRPC wire level): Verify StartSession returns ALREADY_EXISTS
    // gRPC status code when a session is already active.
    // Addresses review finding: gRPC mapping not tested at wire level.
    #[tokio::test]
    async fn test_start_session_already_exists_grpc_status() {
        let (tx, rx) = mpsc::channel(16);
        let svc = ParkingAdaptorService::new(tx);

        tokio::spawn(handle_manual_start(
            rx,
            Err(ManualStartError::AlreadyExists("sess-existing".to_string())),
        ));

        let request = tonic::Request::new(StartSessionRequest {
            vehicle_id: String::new(),
            zone_id: "zone-b".to_string(),
        });

        let result = ParkingAdaptor::start_session(&svc, request).await;
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(
            status.code(),
            tonic::Code::AlreadyExists,
            "StartSession on active session should return ALREADY_EXISTS"
        );
        assert!(
            status.message().contains("sess-existing"),
            "error message should contain existing session_id"
        );
    }

    // TS-08-E2 (gRPC wire level): Verify StopSession returns FAILED_PRECONDITION
    // gRPC status code when no session is active.
    // Addresses review finding: gRPC mapping not tested at wire level.
    #[tokio::test]
    async fn test_stop_session_no_active_grpc_status() {
        let (tx, rx) = mpsc::channel(16);
        let svc = ParkingAdaptorService::new(tx);

        tokio::spawn(handle_manual_stop(
            rx,
            Err(ManualStopError::NoActiveSession),
        ));

        let request = tonic::Request::new(StopSessionRequest {
            session_id: String::new(),
        });

        let result = ParkingAdaptor::stop_session(&svc, request).await;
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(
            status.code(),
            tonic::Code::FailedPrecondition,
            "StopSession with no active session should return FAILED_PRECONDITION"
        );
    }

    // Verify StartSession returns UNAVAILABLE when operator call fails.
    #[tokio::test]
    async fn test_start_session_operator_unavailable_grpc_status() {
        let (tx, rx) = mpsc::channel(16);
        let svc = ParkingAdaptorService::new(tx);

        tokio::spawn(handle_manual_start(
            rx,
            Err(ManualStartError::OperatorFailed(
                OperatorError::RequestFailed("connection refused".to_string()),
            )),
        ));

        let request = tonic::Request::new(StartSessionRequest {
            vehicle_id: String::new(),
            zone_id: "zone-a".to_string(),
        });

        let result = ParkingAdaptor::start_session(&svc, request).await;
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(
            status.code(),
            tonic::Code::Unavailable,
            "StartSession with operator failure should return UNAVAILABLE"
        );
    }

    // Verify StopSession returns UNAVAILABLE when operator call fails.
    #[tokio::test]
    async fn test_stop_session_operator_unavailable_grpc_status() {
        let (tx, rx) = mpsc::channel(16);
        let svc = ParkingAdaptorService::new(tx);

        tokio::spawn(handle_manual_stop(
            rx,
            Err(ManualStopError::OperatorFailed(
                OperatorError::RequestFailed("connection refused".to_string()),
            )),
        ));

        let request = tonic::Request::new(StopSessionRequest {
            session_id: String::new(),
        });

        let result = ParkingAdaptor::stop_session(&svc, request).await;
        assert!(result.is_err());
        let status = result.unwrap_err();
        assert_eq!(
            status.code(),
            tonic::Code::Unavailable,
            "StopSession with operator failure should return UNAVAILABLE"
        );
    }

    // Verify StartSession returns correct response fields on success.
    #[tokio::test]
    async fn test_start_session_success_grpc_response() {
        let (tx, rx) = mpsc::channel(16);
        let svc = ParkingAdaptorService::new(tx);

        tokio::spawn(handle_manual_start(
            rx,
            Ok(StartResponse {
                session_id: "sess-42".to_string(),
                status: "active".to_string(),
                rate: RateResponse {
                    rate_type: "per_hour".to_string(),
                    amount: 3.0,
                    currency: "USD".to_string(),
                },
            }),
        ));

        let request = tonic::Request::new(StartSessionRequest {
            vehicle_id: String::new(),
            zone_id: "zone-a".to_string(),
        });

        let result = ParkingAdaptor::start_session(&svc, request).await;
        assert!(result.is_ok());
        let resp = result.unwrap().into_inner();
        assert_eq!(resp.session_id, "sess-42");
        assert_eq!(resp.status, "active");
        let rate = resp.rate.expect("rate should be present");
        assert_eq!(rate.rate_type, "per_hour");
        assert!((rate.amount - 3.0).abs() < f64::EPSILON);
        assert_eq!(rate.currency, "USD");
    }

    // Verify StopSession returns correct response fields on success.
    #[tokio::test]
    async fn test_stop_session_success_grpc_response() {
        let (tx, rx) = mpsc::channel(16);
        let svc = ParkingAdaptorService::new(tx);

        tokio::spawn(handle_manual_stop(
            rx,
            Ok(StopResponse {
                session_id: "sess-42".to_string(),
                status: "completed".to_string(),
                duration_seconds: 7200,
                total_amount: 6.0,
                currency: "USD".to_string(),
            }),
        ));

        let request = tonic::Request::new(StopSessionRequest {
            session_id: String::new(),
        });

        let result = ParkingAdaptor::stop_session(&svc, request).await;
        assert!(result.is_ok());
        let resp = result.unwrap().into_inner();
        assert_eq!(resp.session_id, "sess-42");
        assert_eq!(resp.status, "completed");
        assert_eq!(resp.duration_seconds, 7200);
        assert!((resp.total_amount - 6.0).abs() < f64::EPSILON);
        assert_eq!(resp.currency, "USD");
    }

    // Verify GetStatus returns session details via gRPC wire level.
    #[tokio::test]
    async fn test_get_status_active_grpc_response() {
        let (tx, mut rx) = mpsc::channel(16);
        let svc = ParkingAdaptorService::new(tx);

        tokio::spawn(async move {
            if let Some(SessionEvent::QueryStatus { reply }) = rx.recv().await {
                let _ = reply.send(Some(crate::session::SessionState {
                    session_id: "sess-1".to_string(),
                    zone_id: "zone-a".to_string(),
                    start_time: 1_700_000_000,
                    rate: crate::session::Rate {
                        rate_type: "per_hour".to_string(),
                        amount: 2.5,
                        currency: "EUR".to_string(),
                    },
                    active: true,
                }));
            }
        });

        let request = tonic::Request::new(GetStatusRequest {});
        let result = ParkingAdaptor::get_status(&svc, request).await;
        assert!(result.is_ok());
        let resp = result.unwrap().into_inner();
        assert!(resp.active);
        assert_eq!(resp.session_id, "sess-1");
        assert_eq!(resp.zone_id, "zone-a");
        assert_eq!(resp.start_time, 1_700_000_000);
        let rate = resp.rate.expect("rate should be present in GetStatus response");
        assert_eq!(rate.rate_type, "per_hour");
        assert!((rate.amount - 2.5).abs() < f64::EPSILON);
        assert_eq!(rate.currency, "EUR");
    }

    // Verify GetStatus returns inactive with empty fields when no session.
    #[tokio::test]
    async fn test_get_status_inactive_grpc_response() {
        let (tx, mut rx) = mpsc::channel(16);
        let svc = ParkingAdaptorService::new(tx);

        tokio::spawn(async move {
            if let Some(SessionEvent::QueryStatus { reply }) = rx.recv().await {
                let _ = reply.send(None);
            }
        });

        let request = tonic::Request::new(GetStatusRequest {});
        let result = ParkingAdaptor::get_status(&svc, request).await;
        assert!(result.is_ok());
        let resp = result.unwrap().into_inner();
        assert!(!resp.active);
        assert_eq!(resp.session_id, "");
        assert_eq!(resp.zone_id, "");
        assert!(resp.rate.is_none());
    }

    // Verify GetRate returns rate via gRPC wire level.
    #[tokio::test]
    async fn test_get_rate_active_grpc_response() {
        let (tx, mut rx) = mpsc::channel(16);
        let svc = ParkingAdaptorService::new(tx);

        tokio::spawn(async move {
            if let Some(SessionEvent::QueryRate { reply }) = rx.recv().await {
                let _ = reply.send(Some(crate::session::Rate {
                    rate_type: "flat_fee".to_string(),
                    amount: 5.0,
                    currency: "EUR".to_string(),
                }));
            }
        });

        let request = tonic::Request::new(GetRateRequest {});
        let result = ParkingAdaptor::get_rate(&svc, request).await;
        assert!(result.is_ok());
        let resp = result.unwrap().into_inner();
        assert_eq!(resp.rate_type, "flat_fee");
        assert!((resp.amount - 5.0).abs() < f64::EPSILON);
        assert_eq!(resp.currency, "EUR");
    }

    // Verify GetRate returns empty rate when no session.
    #[tokio::test]
    async fn test_get_rate_inactive_grpc_response() {
        let (tx, mut rx) = mpsc::channel(16);
        let svc = ParkingAdaptorService::new(tx);

        tokio::spawn(async move {
            if let Some(SessionEvent::QueryRate { reply }) = rx.recv().await {
                let _ = reply.send(None);
            }
        });

        let request = tonic::Request::new(GetRateRequest {});
        let result = ParkingAdaptor::get_rate(&svc, request).await;
        assert!(result.is_ok());
        let resp = result.unwrap().into_inner();
        assert_eq!(resp.rate_type, "");
        assert!((resp.amount - 0.0).abs() < f64::EPSILON);
        assert_eq!(resp.currency, "");
    }
}
