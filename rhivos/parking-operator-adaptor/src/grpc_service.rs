//! ParkingAdaptor gRPC service implementation.
//!
//! Implements the `ParkingAdaptor` gRPC trait, delegating to the operator
//! REST client for actual parking operations and using the session manager
//! to track session state. When a DATA_BROKER client is configured, manual
//! StartSession/StopSession also update `Vehicle.Parking.SessionActive`.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use crate::databroker_client::{signals, DataBrokerClient, DataValue};
use crate::operator_client::{OperatorClient, OperatorError};
use crate::proto::adaptor::parking_adaptor_server::ParkingAdaptor;
use crate::proto::adaptor::{
    GetRateRequest, GetRateResponse, GetStatusRequest, GetStatusResponse, StartSessionRequest,
    StartSessionResponse, StopSessionRequest, StopSessionResponse,
};
use crate::session_manager::SessionManager;
use tonic::{Request, Response, Status};

/// Implementation of the ParkingAdaptor gRPC service.
///
/// Delegates parking operations to the PARKING_OPERATOR REST API via
/// `OperatorClient` and tracks session state via `SessionManager`.
/// When a DATA_BROKER client is configured, manual StartSession/StopSession
/// also write `Vehicle.Parking.SessionActive` to the DATA_BROKER
/// (04-REQ-2.5: override updates SessionActive).
pub struct ParkingAdaptorService {
    /// REST client for the PARKING_OPERATOR.
    operator: OperatorClient,
    /// Session state manager (thread-safe).
    session_mgr: Arc<Mutex<SessionManager>>,
    /// Optional DATA_BROKER client for writing SessionActive on overrides.
    databroker: Option<Arc<dyn DataBrokerClient>>,
}

impl ParkingAdaptorService {
    /// Create a new ParkingAdaptorService without DATA_BROKER integration.
    pub fn new(operator: OperatorClient, session_mgr: Arc<Mutex<SessionManager>>) -> Self {
        ParkingAdaptorService {
            operator,
            session_mgr,
            databroker: None,
        }
    }

    /// Create a new ParkingAdaptorService with DATA_BROKER integration.
    ///
    /// When a DATA_BROKER client is provided, manual StartSession/StopSession
    /// calls will also write `Vehicle.Parking.SessionActive` to DATA_BROKER,
    /// implementing the override behavior defined in 04-REQ-2.5.
    pub fn with_databroker(
        operator: OperatorClient,
        session_mgr: Arc<Mutex<SessionManager>>,
        databroker: Arc<dyn DataBrokerClient>,
    ) -> Self {
        ParkingAdaptorService {
            operator,
            session_mgr,
            databroker: Some(databroker),
        }
    }

    /// Write SessionActive to DATA_BROKER if a client is configured.
    async fn write_session_active(&self, active: bool) {
        if let Some(ref db) = self.databroker {
            if let Err(e) = db
                .write(signals::SESSION_ACTIVE, DataValue::Bool(active))
                .await
            {
                eprintln!(
                    "grpc_service: failed to write SessionActive={}: {}",
                    active, e
                );
            }
        }
    }
}

/// Helper to get the current Unix timestamp.
fn now_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Map an OperatorError to a gRPC Status.
fn operator_error_to_status(err: OperatorError) -> Status {
    match err {
        OperatorError::Unreachable(msg) => Status::unavailable(format!(
            "parking operator unreachable: {}",
            msg
        )),
        OperatorError::NotFound(msg) => Status::not_found(msg),
        OperatorError::Other(msg) => Status::internal(msg),
    }
}

#[tonic::async_trait]
impl ParkingAdaptor for ParkingAdaptorService {
    /// Start a parking session.
    ///
    /// Calls the PARKING_OPERATOR's `POST /parking/start` endpoint.
    /// Returns `ALREADY_EXISTS` if a session is already active.
    /// Returns `UNAVAILABLE` if the operator is unreachable.
    /// When DATA_BROKER is configured, writes `SessionActive = true`.
    async fn start_session(
        &self,
        request: Request<StartSessionRequest>,
    ) -> Result<Response<StartSessionResponse>, Status> {
        let req = request.into_inner();

        // Check if a session is already active
        {
            let mgr = self.session_mgr.lock().await;
            if mgr.has_active_session() {
                return Err(Status::already_exists(
                    "a parking session is already in progress",
                ));
            }
        }

        // Call the operator REST API
        let timestamp = now_timestamp();
        let result = self
            .operator
            .start_session(&req.vehicle_id, &req.zone_id, timestamp)
            .await
            .map_err(operator_error_to_status)?;

        // Register the session in the session manager
        {
            let mut mgr = self.session_mgr.lock().await;
            // Use is_override=true since this is a gRPC call (manual override)
            if let Err(e) = mgr.start_session(
                result.session_id.clone(),
                req.zone_id,
                timestamp,
                true,
            ) {
                // Should not happen since we checked above, but handle gracefully
                return Err(Status::already_exists(e));
            }
        }

        // 04-REQ-2.5: Write SessionActive = true to DATA_BROKER on override
        self.write_session_active(true).await;

        Ok(Response::new(StartSessionResponse {
            session_id: result.session_id,
            status: result.status,
        }))
    }

    /// Stop a parking session.
    ///
    /// Calls the PARKING_OPERATOR's `POST /parking/stop` endpoint.
    /// Returns `NOT_FOUND` if the session_id is unknown.
    /// Returns `UNAVAILABLE` if the operator is unreachable.
    /// When DATA_BROKER is configured, writes `SessionActive = false`.
    async fn stop_session(
        &self,
        request: Request<StopSessionRequest>,
    ) -> Result<Response<StopSessionResponse>, Status> {
        let req = request.into_inner();

        // Check if the session exists in the session manager
        {
            let mgr = self.session_mgr.lock().await;
            match mgr.current_session_id() {
                Some(id) if id == req.session_id => {}
                _ => {
                    return Err(Status::not_found(format!(
                        "no active session with id {}",
                        req.session_id
                    )));
                }
            }
        }

        // Call the operator REST API
        let result = self
            .operator
            .stop_session(&req.session_id)
            .await
            .map_err(operator_error_to_status)?;

        // Remove the session from the session manager
        {
            let mut mgr = self.session_mgr.lock().await;
            let _ = mgr.stop_session(&req.session_id);
        }

        // 04-REQ-2.5: Write SessionActive = false to DATA_BROKER on override
        self.write_session_active(false).await;

        Ok(Response::new(StopSessionResponse {
            session_id: result.session_id,
            total_fee: result.fee,
            duration_seconds: result.duration_seconds,
            currency: result.currency,
        }))
    }

    /// Get the status of a parking session.
    ///
    /// Calls the PARKING_OPERATOR's `GET /parking/{session_id}/status` endpoint.
    /// Returns `NOT_FOUND` if the session_id is unknown.
    async fn get_status(
        &self,
        request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        let req = request.into_inner();

        let result = self
            .operator
            .get_status(&req.session_id)
            .await
            .map_err(operator_error_to_status)?;

        Ok(Response::new(GetStatusResponse {
            session_id: result.session_id,
            active: result.active,
            start_time: result.start_time,
            current_fee: result.current_fee,
            currency: result.currency,
        }))
    }

    /// Get the parking rate for a zone.
    ///
    /// Calls the PARKING_OPERATOR's `GET /rate/{zone_id}` endpoint.
    async fn get_rate(
        &self,
        request: Request<GetRateRequest>,
    ) -> Result<Response<GetRateResponse>, Status> {
        let req = request.into_inner();

        let result = self
            .operator
            .get_rate(&req.zone_id)
            .await
            .map_err(operator_error_to_status)?;

        Ok(Response::new(GetRateResponse {
            rate_per_hour: result.rate_per_hour,
            currency: result.currency,
            zone_name: result.zone_name,
        }))
    }
}
