//! gRPC server implementing the `ParkingAdapter` service.
//!
//! Exposes RPCs for manual session control and status queries.
//! Shares the [`SessionState`] with the lock watcher so both paths
//! see a consistent view of the current parking session.
//!
//! # Requirements
//!
//! - 04-REQ-2.1: Expose gRPC server implementing `ParkingAdapter` service.
//! - 04-REQ-2.2: `StartSession` — start via operator, return session info.
//! - 04-REQ-2.3: `StopSession` — stop active session, return fee summary.
//! - 04-REQ-2.4: `GetStatus` — return current session state.
//! - 04-REQ-2.5: `GetRate` — query operator's rate endpoint.
//! - 04-REQ-2.E1: `StartSession` while active returns existing session.
//! - 04-REQ-2.E2: `StopSession` with unknown/no session returns NOT_FOUND.

use std::time::{SystemTime, UNIX_EPOCH};

use tonic::{Request, Response, Status};
use tracing::{error, info};

use crate::config::Config;
use crate::lock_watcher::SessionState;
use crate::operator_client::OperatorClient;
use crate::session::{ParkingSession, RateType, SessionStatus};

use parking_proto::kuksa_client::KuksaClient;
use parking_proto::services::adapter::parking_adapter_server::ParkingAdapter;
use parking_proto::services::adapter::{
    GetRateRequest, GetRateResponse, GetStatusRequest, GetStatusResponse, StartSessionRequest,
    StartSessionResponse, StopSessionRequest, StopSessionResponse,
};
use parking_proto::signals;

/// Implementation of the `ParkingAdapter` gRPC service.
pub struct ParkingAdapterService {
    session_state: SessionState,
    operator: OperatorClient,
    kuksa: Option<KuksaClient>,
    config: Config,
}

impl ParkingAdapterService {
    pub fn new(
        session_state: SessionState,
        operator: OperatorClient,
        kuksa: Option<KuksaClient>,
        config: Config,
    ) -> Self {
        Self {
            session_state,
            operator,
            kuksa,
            config,
        }
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[tonic::async_trait]
impl ParkingAdapter for ParkingAdapterService {
    async fn start_session(
        &self,
        request: Request<StartSessionRequest>,
    ) -> Result<Response<StartSessionResponse>, Status> {
        let req = request.into_inner();
        let vehicle_vin = req
            .vehicle_id
            .map(|v| v.vin)
            .unwrap_or_else(|| self.config.vehicle_vin.clone());
        let zone_id = if req.zone_id.is_empty() {
            self.config.zone_id.clone()
        } else {
            req.zone_id
        };
        let timestamp = if req.timestamp == 0 {
            now_unix()
        } else {
            req.timestamp
        };

        // 04-REQ-2.E1: if already active, return existing session
        {
            let state = self.session_state.lock().await;
            if let Some(ref session) = *state {
                if session.is_active() {
                    info!(session_id = %session.session_id, "session already active, returning existing");
                    return Ok(Response::new(StartSessionResponse {
                        session_id: session.session_id.clone(),
                        status: "active".into(),
                    }));
                }
            }
        }

        let resp = self
            .operator
            .start_session(&vehicle_vin, &zone_id, timestamp)
            .await
            .map_err(|e| {
                error!(error = %e, "operator start_session failed");
                Status::internal(format!("operator error: {e}"))
            })?;

        let session = ParkingSession {
            session_id: resp.session_id.clone(),
            vehicle_id: vehicle_vin,
            zone_id,
            start_time: timestamp,
            end_time: None,
            rate_type: RateType::from_str_loose(&resp.rate.rate_type),
            rate_amount: resp.rate.rate_amount,
            currency: resp.rate.currency.clone(),
            total_fee: None,
            status: SessionStatus::Active,
        };

        {
            let mut state = self.session_state.lock().await;
            *state = Some(session);
        }

        if let Some(ref kuksa) = self.kuksa {
            if let Err(e) = kuksa
                .set_bool(signals::PARKING_SESSION_ACTIVE, true)
                .await
            {
                error!(error = %e, "failed to write SessionActive=true");
            }
        }

        info!(session_id = %resp.session_id, "session started via gRPC");
        Ok(Response::new(StartSessionResponse {
            session_id: resp.session_id,
            status: resp.status,
        }))
    }

    async fn stop_session(
        &self,
        request: Request<StopSessionRequest>,
    ) -> Result<Response<StopSessionResponse>, Status> {
        let req = request.into_inner();
        let timestamp = if req.timestamp == 0 {
            now_unix()
        } else {
            req.timestamp
        };

        // 04-REQ-2.E2: unknown or inactive session → NOT_FOUND
        let session_id = {
            let state = self.session_state.lock().await;
            match state.as_ref() {
                Some(s)
                    if s.is_active()
                        && (req.session_id.is_empty() || s.session_id == req.session_id) =>
                {
                    s.session_id.clone()
                }
                _ => {
                    return Err(Status::not_found(format!(
                        "no active session found for id '{}'",
                        req.session_id
                    )));
                }
            }
        };

        let resp = self
            .operator
            .stop_session(&session_id, timestamp)
            .await
            .map_err(|e| {
                error!(error = %e, "operator stop_session failed");
                Status::internal(format!("operator error: {e}"))
            })?;

        {
            let mut state = self.session_state.lock().await;
            if let Some(ref mut s) = *state {
                s.complete(timestamp, resp.total_fee, resp.duration_seconds);
            }
        }

        if let Some(ref kuksa) = self.kuksa {
            if let Err(e) = kuksa
                .set_bool(signals::PARKING_SESSION_ACTIVE, false)
                .await
            {
                error!(error = %e, "failed to write SessionActive=false");
            }
        }

        info!(session_id = %session_id, total_fee = resp.total_fee, "session stopped via gRPC");
        Ok(Response::new(StopSessionResponse {
            status: resp.status,
            total_fee: resp.total_fee,
            duration_seconds: resp.duration_seconds,
        }))
    }

    async fn get_status(
        &self,
        request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        let req = request.into_inner();
        let state = self.session_state.lock().await;

        match state.as_ref() {
            Some(session)
                if req.session_id.is_empty() || session.session_id == req.session_id =>
            {
                let now = now_unix();
                Ok(Response::new(GetStatusResponse {
                    session_id: session.session_id.clone(),
                    active: session.is_active(),
                    start_time: session.start_time,
                    current_fee: session.current_fee(now),
                }))
            }
            _ => Err(Status::not_found(format!(
                "no session found for id '{}'",
                req.session_id
            ))),
        }
    }

    async fn get_rate(
        &self,
        request: Request<GetRateRequest>,
    ) -> Result<Response<GetRateResponse>, Status> {
        let req = request.into_inner();
        let zone_id = if req.zone_id.is_empty() {
            &self.config.zone_id
        } else {
            &req.zone_id
        };

        let resp = self.operator.get_rate(zone_id).await.map_err(|e| {
            error!(error = %e, "operator get_rate failed");
            Status::internal(format!("operator error: {e}"))
        })?;

        Ok(Response::new(GetRateResponse {
            zone_id: resp.zone_id,
            rate_per_hour: resp.rate_amount,
            currency: resp.currency,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_service(operator_url: &str) -> ParkingAdapterService {
        let config = Config {
            listen_addr: "0.0.0.0:50054".into(),
            databroker_addr: "http://localhost:55555".into(),
            parking_operator_url: operator_url.into(),
            zone_id: "zone-1".into(),
            vehicle_vin: "DEMO0000000000001".into(),
        };
        ParkingAdapterService::new(
            Arc::new(Mutex::new(None)),
            OperatorClient::new(operator_url),
            None,
            config,
        )
    }

    #[tokio::test]
    async fn start_session_calls_operator() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "sess-001",
                "status": "active",
                "rate": {
                    "zone_id": "zone-1",
                    "rate_type": "per_minute",
                    "rate_amount": 0.05,
                    "currency": "EUR"
                }
            })))
            .mount(&mock)
            .await;

        let svc = make_service(&mock.uri());
        let req = Request::new(StartSessionRequest {
            vehicle_id: Some(parking_proto::common::VehicleId {
                vin: "DEMO0000000000001".into(),
            }),
            zone_id: "zone-1".into(),
            timestamp: 1_708_300_800,
        });

        let resp = svc.start_session(req).await.unwrap().into_inner();
        assert_eq!(resp.session_id, "sess-001");
        assert_eq!(resp.status, "active");
    }

    #[tokio::test]
    async fn start_session_while_active_returns_existing() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "sess-001",
                "status": "active",
                "rate": {
                    "zone_id": "zone-1",
                    "rate_type": "per_minute",
                    "rate_amount": 0.05,
                    "currency": "EUR"
                }
            })))
            .expect(1)
            .mount(&mock)
            .await;

        let svc = make_service(&mock.uri());

        let req1 = Request::new(StartSessionRequest {
            vehicle_id: Some(parking_proto::common::VehicleId {
                vin: "DEMO0000000000001".into(),
            }),
            zone_id: "zone-1".into(),
            timestamp: 1_708_300_800,
        });
        let resp1 = svc.start_session(req1).await.unwrap().into_inner();
        assert_eq!(resp1.session_id, "sess-001");

        let req2 = Request::new(StartSessionRequest {
            vehicle_id: Some(parking_proto::common::VehicleId {
                vin: "DEMO0000000000001".into(),
            }),
            zone_id: "zone-1".into(),
            timestamp: 1_708_300_900,
        });
        let resp2 = svc.start_session(req2).await.unwrap().into_inner();
        assert_eq!(resp2.session_id, "sess-001");
        assert_eq!(resp2.status, "active");
    }

    #[tokio::test]
    async fn stop_session_unknown_returns_not_found() {
        let mock = MockServer::start().await;
        let svc = make_service(&mock.uri());

        let req = Request::new(StopSessionRequest {
            session_id: "nonexistent".into(),
            timestamp: 1_708_301_000,
        });
        let err = svc.stop_session(req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn get_status_no_session_returns_not_found() {
        let mock = MockServer::start().await;
        let svc = make_service(&mock.uri());

        let req = Request::new(GetStatusRequest {
            session_id: "nonexistent".into(),
        });
        let err = svc.get_status(req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn get_rate_calls_operator() {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/parking/rate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "zone_id": "zone-1",
                "rate_type": "per_minute",
                "rate_amount": 0.05,
                "currency": "EUR"
            })))
            .mount(&mock)
            .await;

        let svc = make_service(&mock.uri());
        let req = Request::new(GetRateRequest {
            zone_id: "zone-1".into(),
        });

        let resp = svc.get_rate(req).await.unwrap().into_inner();
        assert_eq!(resp.zone_id, "zone-1");
        assert!((resp.rate_per_hour - 0.05).abs() < f64::EPSILON);
        assert_eq!(resp.currency, "EUR");
    }

    #[tokio::test]
    async fn full_session_lifecycle() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "sess-lc",
                "status": "active",
                "rate": {
                    "zone_id": "zone-1",
                    "rate_type": "per_minute",
                    "rate_amount": 0.05,
                    "currency": "EUR"
                }
            })))
            .mount(&mock)
            .await;

        Mock::given(method("POST"))
            .and(path("/parking/stop"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "sess-lc",
                "status": "completed",
                "total_fee": 0.25,
                "duration_seconds": 300,
                "currency": "EUR"
            })))
            .mount(&mock)
            .await;

        let svc = make_service(&mock.uri());

        // Start session
        let req = Request::new(StartSessionRequest {
            vehicle_id: Some(parking_proto::common::VehicleId {
                vin: "DEMO0000000000001".into(),
            }),
            zone_id: "zone-1".into(),
            timestamp: 1_708_300_800,
        });
        let resp = svc.start_session(req).await.unwrap().into_inner();
        assert_eq!(resp.session_id, "sess-lc");

        // GetStatus while active
        let req = Request::new(GetStatusRequest {
            session_id: "sess-lc".into(),
        });
        let status = svc.get_status(req).await.unwrap().into_inner();
        assert!(status.active);

        // Stop session
        let req = Request::new(StopSessionRequest {
            session_id: "sess-lc".into(),
            timestamp: 1_708_301_100,
        });
        let resp = svc.stop_session(req).await.unwrap().into_inner();
        assert_eq!(resp.status, "completed");
        assert!((resp.total_fee - 0.25).abs() < f64::EPSILON);
        assert_eq!(resp.duration_seconds, 300);

        // GetStatus after completion
        let req = Request::new(GetStatusRequest {
            session_id: "sess-lc".into(),
        });
        let status = svc.get_status(req).await.unwrap().into_inner();
        assert!(!status.active);
    }

    // ── Tests with mock Kuksa to verify SessionActive signal writes ──────
    //
    // **Property 8 (SessionActive Signal Accuracy):** The gRPC server MUST
    // write SessionActive=true on StartSession and SessionActive=false on
    // StopSession via DATA_BROKER.

    use parking_proto::kuksa::val::v2 as proto;
    use parking_proto::kuksa::val::v2::val_server::{Val, ValServer};

    /// Minimal mock Kuksa VAL server that records PublishValue calls.
    #[derive(Debug)]
    struct MockKuksaVal {
        published: Arc<Mutex<Vec<(String, bool)>>>,
    }

    impl MockKuksaVal {
        fn new() -> Self {
            Self {
                published: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[tonic::async_trait]
    impl Val for MockKuksaVal {
        async fn get_value(
            &self,
            _req: tonic::Request<proto::GetValueRequest>,
        ) -> Result<tonic::Response<proto::GetValueResponse>, tonic::Status> {
            Err(tonic::Status::unimplemented("unused"))
        }

        async fn get_values(
            &self,
            _req: tonic::Request<proto::GetValuesRequest>,
        ) -> Result<tonic::Response<proto::GetValuesResponse>, tonic::Status> {
            Err(tonic::Status::unimplemented("unused"))
        }

        type SubscribeStream = tokio_stream::wrappers::ReceiverStream<
            Result<proto::SubscribeResponse, tonic::Status>,
        >;

        async fn subscribe(
            &self,
            _req: tonic::Request<proto::SubscribeRequest>,
        ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            Ok(tonic::Response::new(
                tokio_stream::wrappers::ReceiverStream::new(rx),
            ))
        }

        type SubscribeByIdStream = tokio_stream::wrappers::ReceiverStream<
            Result<proto::SubscribeByIdResponse, tonic::Status>,
        >;

        async fn subscribe_by_id(
            &self,
            _req: tonic::Request<proto::SubscribeByIdRequest>,
        ) -> Result<tonic::Response<Self::SubscribeByIdStream>, tonic::Status> {
            Err(tonic::Status::unimplemented("unused"))
        }

        async fn actuate(
            &self,
            _req: tonic::Request<proto::ActuateRequest>,
        ) -> Result<tonic::Response<proto::ActuateResponse>, tonic::Status> {
            Err(tonic::Status::unimplemented("unused"))
        }

        async fn batch_actuate(
            &self,
            _req: tonic::Request<proto::BatchActuateRequest>,
        ) -> Result<tonic::Response<proto::BatchActuateResponse>, tonic::Status> {
            Err(tonic::Status::unimplemented("unused"))
        }

        async fn list_metadata(
            &self,
            _req: tonic::Request<proto::ListMetadataRequest>,
        ) -> Result<tonic::Response<proto::ListMetadataResponse>, tonic::Status> {
            Err(tonic::Status::unimplemented("unused"))
        }

        async fn publish_value(
            &self,
            req: tonic::Request<proto::PublishValueRequest>,
        ) -> Result<tonic::Response<proto::PublishValueResponse>, tonic::Status> {
            let inner = req.into_inner();
            if let Some(signal_id) = inner.signal_id {
                if let Some(proto::signal_id::Signal::Path(path)) = signal_id.signal {
                    if let Some(dp) = inner.data_point {
                        if let Some(val) = dp.value {
                            if let Some(proto::value::TypedValue::Bool(b)) = val.typed_value {
                                self.published.lock().await.push((path, b));
                            }
                        }
                    }
                }
            }
            Ok(tonic::Response::new(proto::PublishValueResponse {}))
        }

        type OpenProviderStreamStream = tokio_stream::wrappers::ReceiverStream<
            Result<proto::OpenProviderStreamResponse, tonic::Status>,
        >;

        async fn open_provider_stream(
            &self,
            _req: tonic::Request<tonic::Streaming<proto::OpenProviderStreamRequest>>,
        ) -> Result<tonic::Response<Self::OpenProviderStreamStream>, tonic::Status> {
            Err(tonic::Status::unimplemented("unused"))
        }

        async fn get_server_info(
            &self,
            _req: tonic::Request<proto::GetServerInfoRequest>,
        ) -> Result<tonic::Response<proto::GetServerInfoResponse>, tonic::Status> {
            Ok(tonic::Response::new(proto::GetServerInfoResponse {
                name: "mock".into(),
                version: "test".into(),
                commit_hash: "".into(),
            }))
        }
    }

    /// Start a mock Kuksa gRPC server and return (client, published_log).
    async fn start_mock_kuksa() -> (KuksaClient, Arc<Mutex<Vec<(String, bool)>>>) {
        let mock = MockKuksaVal::new();
        let published = mock.published.clone();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(ValServer::new(mock))
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
                .await
                .unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = KuksaClient::connect(&format!("http://{}", addr))
            .await
            .expect("connect to mock kuksa");

        (client, published)
    }

    fn make_service_with_kuksa(
        operator_url: &str,
        kuksa: KuksaClient,
    ) -> ParkingAdapterService {
        let config = Config {
            listen_addr: "0.0.0.0:50054".into(),
            databroker_addr: "http://localhost:55555".into(),
            parking_operator_url: operator_url.into(),
            zone_id: "zone-1".into(),
            vehicle_vin: "DEMO0000000000001".into(),
        };
        ParkingAdapterService::new(
            Arc::new(Mutex::new(None)),
            OperatorClient::new(operator_url),
            Some(kuksa),
            config,
        )
    }

    /// **Property 8:** StartSession via gRPC writes SessionActive=true to Kuksa.
    #[tokio::test]
    async fn start_session_writes_session_active_true() {
        let (kuksa, published) = start_mock_kuksa().await;
        let mock_op = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "sess-sig",
                "status": "active",
                "rate": {
                    "zone_id": "zone-1",
                    "rate_type": "per_minute",
                    "rate_amount": 0.05,
                    "currency": "EUR"
                }
            })))
            .mount(&mock_op)
            .await;

        let svc = make_service_with_kuksa(&mock_op.uri(), kuksa);

        let req = Request::new(StartSessionRequest {
            vehicle_id: Some(parking_proto::common::VehicleId {
                vin: "DEMO0000000000001".into(),
            }),
            zone_id: "zone-1".into(),
            timestamp: 1_708_300_800,
        });
        let resp = svc.start_session(req).await.unwrap().into_inner();
        assert_eq!(resp.session_id, "sess-sig");

        // Verify SessionActive=true was written to Kuksa
        let writes = published.lock().await;
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].0, signals::PARKING_SESSION_ACTIVE);
        assert!(writes[0].1, "SessionActive should be true after StartSession");
    }

    /// **Property 8:** StopSession via gRPC writes SessionActive=false to Kuksa.
    #[tokio::test]
    async fn stop_session_writes_session_active_false() {
        let (kuksa, published) = start_mock_kuksa().await;
        let mock_op = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "sess-sig2",
                "status": "active",
                "rate": {
                    "zone_id": "zone-1",
                    "rate_type": "per_minute",
                    "rate_amount": 0.05,
                    "currency": "EUR"
                }
            })))
            .mount(&mock_op)
            .await;

        Mock::given(method("POST"))
            .and(path("/parking/stop"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "sess-sig2",
                "status": "completed",
                "total_fee": 0.25,
                "duration_seconds": 300,
                "currency": "EUR"
            })))
            .mount(&mock_op)
            .await;

        let svc = make_service_with_kuksa(&mock_op.uri(), kuksa);

        // Start first
        let req = Request::new(StartSessionRequest {
            vehicle_id: Some(parking_proto::common::VehicleId {
                vin: "DEMO0000000000001".into(),
            }),
            zone_id: "zone-1".into(),
            timestamp: 1_708_300_800,
        });
        svc.start_session(req).await.unwrap();

        // Then stop
        let req = Request::new(StopSessionRequest {
            session_id: "sess-sig2".into(),
            timestamp: 1_708_301_100,
        });
        let resp = svc.stop_session(req).await.unwrap().into_inner();
        assert_eq!(resp.status, "completed");

        // Verify both writes: SessionActive=true then SessionActive=false
        let writes = published.lock().await;
        assert_eq!(writes.len(), 2);
        assert!(writes[0].1, "first write should be true (start)");
        assert!(!writes[1].1, "second write should be false (stop)");
    }

    /// **Property 8:** StartSession while active does NOT write to Kuksa
    /// (session already tracked, signal already true).
    #[tokio::test]
    async fn start_session_while_active_does_not_write_kuksa() {
        let (kuksa, published) = start_mock_kuksa().await;
        let mock_op = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/parking/start"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "sess-dup",
                "status": "active",
                "rate": {
                    "zone_id": "zone-1",
                    "rate_type": "per_minute",
                    "rate_amount": 0.05,
                    "currency": "EUR"
                }
            })))
            .expect(1) // Only one actual call to operator
            .mount(&mock_op)
            .await;

        let svc = make_service_with_kuksa(&mock_op.uri(), kuksa);

        // First start
        let req = Request::new(StartSessionRequest {
            vehicle_id: Some(parking_proto::common::VehicleId {
                vin: "DEMO0000000000001".into(),
            }),
            zone_id: "zone-1".into(),
            timestamp: 1_708_300_800,
        });
        svc.start_session(req).await.unwrap();

        // Second start — should return existing without touching Kuksa again
        let req = Request::new(StartSessionRequest {
            vehicle_id: Some(parking_proto::common::VehicleId {
                vin: "DEMO0000000000001".into(),
            }),
            zone_id: "zone-1".into(),
            timestamp: 1_708_300_900,
        });
        let resp = svc.start_session(req).await.unwrap().into_inner();
        assert_eq!(resp.session_id, "sess-dup");
        assert_eq!(resp.status, "active");

        // Only one write to Kuksa (from the first start)
        let writes = published.lock().await;
        assert_eq!(writes.len(), 1, "duplicate start should not write to Kuksa again");
    }
}
