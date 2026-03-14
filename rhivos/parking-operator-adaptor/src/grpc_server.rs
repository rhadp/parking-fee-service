//! Tonic gRPC trait implementation for the PARKING_OPERATOR_ADAPTOR.
//!
//! [`ParkingAdaptorImpl`] wraps [`crate::grpc_service::ParkingService`] and
//! implements the generated [`parking_adaptor_server::ParkingAdaptor`] trait
//! so that the service can be registered with a `tonic::transport::Server`.
//!
//! All business logic lives in `grpc_service.rs`; this module only maps
//! proto request/response types to the internal Rust types.
//!
//! Requirements: 08-REQ-3.1, 08-REQ-3.2, 08-REQ-3.E1, 08-REQ-3.E2,
//!               08-REQ-4.1, 08-REQ-4.2, 08-REQ-5.1, 08-REQ-5.2

use tonic::{Request, Response, Status};

use crate::grpc_service::ParkingService;
use crate::proto::parking_adaptor::{
    parking_adaptor_server::ParkingAdaptor, GetRateRequest, GetRateResponse, GetStatusRequest,
    GetStatusResponse, Rate as ProtoRate, StartSessionRequest, StartSessionResponse,
    StopSessionRequest, StopSessionResponse,
};

// ---------------------------------------------------------------------------
// Wrapper struct
// ---------------------------------------------------------------------------

/// tonic service implementation for `ParkingAdaptor`.
///
/// Delegates all logic to [`ParkingService`] and maps the results to the
/// proto request/response types generated from `parking_adaptor.proto`.
pub struct ParkingAdaptorImpl {
    inner: ParkingService,
}

impl ParkingAdaptorImpl {
    /// Wrap a [`ParkingService`] for use as a tonic gRPC service.
    pub fn new(service: ParkingService) -> Self {
        Self { inner: service }
    }
}

// ---------------------------------------------------------------------------
// Tonic trait implementation
// ---------------------------------------------------------------------------

#[tonic::async_trait]
impl ParkingAdaptor for ParkingAdaptorImpl {
    // -----------------------------------------------------------------------
    // StartSession — 08-REQ-3.1, 08-REQ-3.E1
    // -----------------------------------------------------------------------

    async fn start_session(
        &self,
        request: Request<StartSessionRequest>,
    ) -> Result<Response<StartSessionResponse>, Status> {
        let zone_id = request.into_inner().zone_id;
        let result = self.inner.start_session(&zone_id).await?;
        Ok(Response::new(StartSessionResponse {
            session_id: result.session_id,
            status: result.status,
        }))
    }

    // -----------------------------------------------------------------------
    // StopSession — 08-REQ-3.2, 08-REQ-3.E2
    // -----------------------------------------------------------------------

    async fn stop_session(
        &self,
        _request: Request<StopSessionRequest>,
    ) -> Result<Response<StopSessionResponse>, Status> {
        let result = self.inner.stop_session().await?;
        Ok(Response::new(StopSessionResponse {
            session_id: result.session_id,
            status: result.status,
            duration_seconds: result.duration_seconds,
            total_amount: result.total_amount,
            currency: result.currency,
        }))
    }

    // -----------------------------------------------------------------------
    // GetStatus — 08-REQ-4.1, 08-REQ-4.2
    // -----------------------------------------------------------------------

    async fn get_status(
        &self,
        _request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        let status = self.inner.get_status().await;
        let resp = match status {
            Some(s) => GetStatusResponse {
                session_id: s.session_id,
                active: true,
                zone_id: s.zone_id,
                start_time: s.start_time,
                rate: Some(ProtoRate {
                    rate_type: s.rate.rate_type,
                    amount: s.rate.amount,
                    currency: s.rate.currency,
                }),
            },
            None => GetStatusResponse {
                session_id: String::new(),
                active: false,
                zone_id: String::new(),
                start_time: 0,
                rate: None,
            },
        };
        Ok(Response::new(resp))
    }

    // -----------------------------------------------------------------------
    // GetRate — 08-REQ-5.1, 08-REQ-5.2
    // -----------------------------------------------------------------------

    async fn get_rate(
        &self,
        _request: Request<GetRateRequest>,
    ) -> Result<Response<GetRateResponse>, Status> {
        let rate = self.inner.get_rate().await?;
        Ok(Response::new(GetRateResponse {
            rate_type: rate.rate_type,
            amount: rate.amount,
            currency: rate.currency,
        }))
    }
}
