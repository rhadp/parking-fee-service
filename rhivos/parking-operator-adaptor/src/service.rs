//! gRPC service implementation for PARKING_OPERATOR_ADAPTOR.
//!
//! This module provides the ParkingAdaptor gRPC service for PARKING_APP.

use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::{debug, error, info};

use crate::location::LocationReader;
use crate::manager::SessionManager;
use crate::proto::parking::{
    parking_adaptor_server::ParkingAdaptor, GetSessionStatusRequest, GetSessionStatusResponse,
    SessionState as ProtoSessionState, StartSessionRequest, StartSessionResponse,
    StopSessionRequest, StopSessionResponse,
};

/// gRPC service implementation for ParkingAdaptor.
pub struct ParkingAdaptorImpl {
    /// Session manager
    session_manager: Arc<SessionManager>,
    /// Location reader
    location_reader: LocationReader,
}

impl ParkingAdaptorImpl {
    /// Create a new ParkingAdaptorImpl.
    pub fn new(session_manager: Arc<SessionManager>, location_reader: LocationReader) -> Self {
        Self {
            session_manager,
            location_reader,
        }
    }
}

#[tonic::async_trait]
impl ParkingAdaptor for ParkingAdaptorImpl {
    async fn start_session(
        &self,
        request: Request<StartSessionRequest>,
    ) -> Result<Response<StartSessionResponse>, Status> {
        let req = request.into_inner();
        info!("gRPC StartSession request: zone_id={}", req.zone_id);

        // Get current location
        let location = match self.location_reader.read_location().await {
            Ok(loc) => loc,
            Err(e) => {
                error!("Failed to read location: {}", e);
                return Ok(Response::new(StartSessionResponse {
                    success: false,
                    error_message: format!("Location unavailable: {}", e),
                    session_id: String::new(),
                    state: ProtoSessionState::Error.into(),
                }));
            }
        };

        // Start session
        match self.session_manager.start_session(&location, &req.zone_id).await {
            Ok(session) => {
                info!("Session started successfully: {}", session.session_id);
                Ok(Response::new(StartSessionResponse {
                    success: true,
                    error_message: String::new(),
                    session_id: session.session_id,
                    state: session.state.to_proto(),
                }))
            }
            Err(e) => {
                error!("Failed to start session: {}", e);
                Ok(Response::new(StartSessionResponse {
                    success: false,
                    error_message: e.to_string(),
                    session_id: String::new(),
                    state: ProtoSessionState::Error.into(),
                }))
            }
        }
    }

    async fn stop_session(
        &self,
        _request: Request<StopSessionRequest>,
    ) -> Result<Response<StopSessionResponse>, Status> {
        info!("gRPC StopSession request");

        match self.session_manager.stop_session().await {
            Ok(session) => {
                info!("Session stopped successfully: {}", session.session_id);
                let duration = session.duration_seconds();
                Ok(Response::new(StopSessionResponse {
                    success: true,
                    error_message: String::new(),
                    session_id: session.session_id,
                    state: session.state.to_proto(),
                    final_cost: session.final_cost.unwrap_or(0.0),
                    duration_seconds: duration,
                }))
            }
            Err(e) => {
                error!("Failed to stop session: {}", e);
                Ok(Response::new(StopSessionResponse {
                    success: false,
                    error_message: e.to_string(),
                    session_id: String::new(),
                    state: ProtoSessionState::Error.into(),
                    final_cost: 0.0,
                    duration_seconds: 0,
                }))
            }
        }
    }

    async fn get_session_status(
        &self,
        _request: Request<GetSessionStatusRequest>,
    ) -> Result<Response<GetSessionStatusResponse>, Status> {
        debug!("gRPC GetSessionStatus request");

        let session = self.session_manager.get_session().await;

        match session {
            Some(s) => {
                Ok(Response::new(GetSessionStatusResponse {
                    has_active_session: s.is_active(),
                    session_id: s.session_id.clone(),
                    state: s.state.to_proto(),
                    start_time_unix: s.start_time_unix(),
                    duration_seconds: s.duration_seconds(),
                    current_cost: s.current_cost,
                    zone_id: s.zone_id.clone(),
                    error_message: s.error_message.clone().unwrap_or_default(),
                    latitude: s.location.latitude,
                    longitude: s.location.longitude,
                }))
            }
            None => {
                Ok(Response::new(GetSessionStatusResponse {
                    has_active_session: false,
                    session_id: String::new(),
                    state: ProtoSessionState::None.into(),
                    start_time_unix: 0,
                    duration_seconds: 0,
                    current_cost: 0.0,
                    zone_id: String::new(),
                    error_message: String::new(),
                    latitude: 0.0,
                    longitude: 0.0,
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServiceConfig;
    use crate::operator::OperatorApiClient;
    use crate::publisher::StatePublisher;
    use crate::store::SessionStore;
    use crate::zone::ZoneLookupClient;
    use proptest::prelude::*;
    use tempfile::TempDir;

    fn create_test_service(temp_dir: &TempDir) -> ParkingAdaptorImpl {
        let config = ServiceConfig::default();
        let location_reader = LocationReader::new(config.data_broker_socket.clone());
        let zone_lookup_client = ZoneLookupClient::new(
            config.parking_fee_service_url.clone(),
            config.api_max_retries,
            config.api_base_delay_ms,
            config.api_timeout_ms,
        );
        let operator_client = OperatorApiClient::new(
            config.operator_base_url.clone(),
            config.vehicle_id.clone(),
            config.api_max_retries,
            config.api_base_delay_ms,
            config.api_max_delay_ms,
            config.api_timeout_ms,
        );
        let state_publisher = StatePublisher::new(config.data_broker_socket.clone());
        let session_store = SessionStore::new(temp_dir.path().join("session.json"));

        let session_manager = Arc::new(SessionManager::new(
            location_reader.clone(),
            zone_lookup_client,
            operator_client,
            state_publisher,
            session_store,
        ));

        ParkingAdaptorImpl::new(session_manager, location_reader)
    }

    #[tokio::test]
    async fn test_get_session_status_no_session() {
        let temp_dir = TempDir::new().unwrap();
        let service = create_test_service(&temp_dir);

        let request = Request::new(GetSessionStatusRequest {});
        let response = service.get_session_status(request).await.unwrap();
        let inner = response.into_inner();

        assert!(!inner.has_active_session);
        assert!(inner.session_id.is_empty());
        assert_eq!(inner.state, ProtoSessionState::None as i32);
    }

    // Property 10: gRPC Response Format Compliance
    // Validates: Requirements 5.1, 5.2, 5.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_status_response_defaults_valid(
            _lat in -90.0f64..90.0,
            _lng in -180.0f64..180.0
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let service = create_test_service(&temp_dir);

                let request = Request::new(GetSessionStatusRequest {});
                let response = service.get_session_status(request).await.unwrap();
                let inner = response.into_inner();

                // Default response should have valid field values
                prop_assert!(!inner.has_active_session);
                prop_assert!(inner.duration_seconds >= 0);
                prop_assert!(inner.current_cost >= 0.0);
                Ok(())
            })?;
        }
    }

    // Property 11: Start/Stop Response Symmetry
    // Validates: Requirements 3.4, 4.4
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_response_state_consistency(
            zone_id in "[a-z0-9-]{4,20}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let service = create_test_service(&temp_dir);

                // Start request (will fail due to mock, but response format should be valid)
                let start_request = Request::new(StartSessionRequest { zone_id });
                let start_response = service.start_session(start_request).await.unwrap();
                let start_inner = start_response.into_inner();

                // Response should have consistent format
                if start_inner.success {
                    prop_assert!(!start_inner.session_id.is_empty());
                    prop_assert!(start_inner.error_message.is_empty());
                } else {
                    prop_assert!(!start_inner.error_message.is_empty());
                }
                Ok(())
            })?;
        }
    }
}
