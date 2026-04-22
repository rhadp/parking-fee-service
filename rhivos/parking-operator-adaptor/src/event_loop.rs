use crate::broker::BrokerClient;
use crate::operator::{OperatorError, RateResponse, StartResponse, StopResponse};
use crate::session::{Rate, Session};

/// Process a lock event (IsLocked changed).
///
/// When IsLocked becomes true and no session is active, starts a session
/// with the operator and publishes SessionActive=true to DATA_BROKER.
///
/// When IsLocked becomes false and a session is active, stops the session
/// with the operator and publishes SessionActive=false to DATA_BROKER.
///
/// Idempotent: lock while active or unlock while inactive are no-ops.
pub async fn process_lock_event<B: BrokerClient, O: OperatorApi>(
    _is_locked: bool,
    _session: &mut Session,
    _operator: &O,
    _broker: &B,
    _vehicle_id: &str,
    _zone_id: &str,
) -> Result<(), OperatorError> {
    todo!("implement process_lock_event")
}

/// Process a manual StartSession request.
///
/// Returns ALREADY_EXISTS-style error if a session is already active.
/// Otherwise starts a session with the operator.
pub async fn process_manual_start<B: BrokerClient, O: OperatorApi>(
    _zone_id: &str,
    _session: &mut Session,
    _operator: &O,
    _broker: &B,
    _vehicle_id: &str,
) -> Result<StartResponse, ManualStartError> {
    todo!("implement process_manual_start")
}

/// Process a manual StopSession request.
///
/// Returns FAILED_PRECONDITION-style error if no session is active.
/// Otherwise stops the session with the operator.
pub async fn process_manual_stop<B: BrokerClient, O: OperatorApi>(
    _session: &mut Session,
    _operator: &O,
    _broker: &B,
) -> Result<StopResponse, ManualStopError> {
    todo!("implement process_manual_stop")
}

/// Error type for manual StartSession.
#[derive(Debug)]
pub enum ManualStartError {
    /// A session is already active.
    AlreadyExists(String),
    /// Operator call failed.
    OperatorFailed(OperatorError),
}

/// Error type for manual StopSession.
#[derive(Debug)]
pub enum ManualStopError {
    /// No session is active.
    NoActiveSession,
    /// Operator call failed.
    OperatorFailed(OperatorError),
}

/// Trait abstracting the operator client for testability.
///
/// Allows mock injection in unit tests without HTTP.
#[allow(async_fn_in_trait)]
pub trait OperatorApi {
    /// Start a parking session with the operator.
    async fn start_session(
        &self,
        vehicle_id: &str,
        zone_id: &str,
    ) -> Result<StartResponse, OperatorError>;

    /// Stop a parking session with the operator.
    async fn stop_session(&self, session_id: &str) -> Result<StopResponse, OperatorError>;
}

/// Convert an operator RateResponse to a session Rate.
#[allow(dead_code)]
fn rate_from_response(r: &RateResponse) -> Rate {
    Rate {
        rate_type: r.rate_type.clone(),
        amount: r.amount,
        currency: r.currency.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::SIGNAL_SESSION_ACTIVE;
    use crate::testing::{
        make_start_response, make_stop_response, MockBrokerClient, MockOperatorClient,
    };

    // TS-08-11: Verify lock event (IsLocked=true) triggers session start.
    #[tokio::test]
    async fn test_lock_event_starts_session() {
        let mock_broker = MockBrokerClient::new();
        let mock_operator = MockOperatorClient::new();
        mock_operator.on_start_return(Ok(make_start_response("sess-1")));

        let mut session = Session::new();

        let result = process_lock_event(
            true,
            &mut session,
            &mock_operator,
            &mock_broker,
            "DEMO-VIN-001",
            "zone-demo-1",
        )
        .await;
        assert!(result.is_ok());

        // Verify operator was called with correct args.
        let calls = mock_operator.start_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "DEMO-VIN-001");
        assert_eq!(calls[0].1, "zone-demo-1");

        // Verify session is active.
        assert!(session.is_active());

        // Verify SessionActive was set to true.
        let broker_calls = mock_broker.set_bool_calls();
        assert!(
            broker_calls
                .iter()
                .any(|(s, v)| s == SIGNAL_SESSION_ACTIVE && *v),
            "should have set SessionActive to true"
        );
    }

    // TS-08-12: Verify unlock event (IsLocked=false) triggers session stop.
    #[tokio::test]
    async fn test_unlock_event_stops_session() {
        let mock_broker = MockBrokerClient::new();
        let mock_operator = MockOperatorClient::new();
        mock_operator.on_stop_return(Ok(make_stop_response("sess-1")));

        let mut session = Session::new();
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        };
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        let result =
            process_lock_event(false, &mut session, &mock_operator, &mock_broker, "VIN", "zone")
                .await;
        assert!(result.is_ok());

        // Verify operator stop was called.
        let calls = mock_operator.stop_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "sess-1");

        // Verify session is inactive.
        assert!(!session.is_active());

        // Verify SessionActive was set to false.
        let broker_calls = mock_broker.set_bool_calls();
        assert!(
            broker_calls
                .iter()
                .any(|(s, v)| s == SIGNAL_SESSION_ACTIVE && !*v),
            "should have set SessionActive to false"
        );
    }

    // TS-08-13: Verify SessionActive is set to true on session start.
    #[tokio::test]
    async fn test_session_active_set_true() {
        let mock_broker = MockBrokerClient::new();
        let mock_operator = MockOperatorClient::new();
        mock_operator.on_start_return(Ok(make_start_response("sess-1")));

        let mut session = Session::new();

        let _ = process_lock_event(
            true,
            &mut session,
            &mock_operator,
            &mock_broker,
            "VIN",
            "zone",
        )
        .await;

        assert!(
            mock_broker
                .set_bool_calls()
                .iter()
                .any(|(s, v)| s == SIGNAL_SESSION_ACTIVE && *v),
            "should have set SessionActive to true"
        );
    }

    // TS-08-14: Verify SessionActive is set to false on session stop.
    #[tokio::test]
    async fn test_session_active_set_false() {
        let mock_broker = MockBrokerClient::new();
        let mock_operator = MockOperatorClient::new();
        mock_operator.on_stop_return(Ok(make_stop_response("sess-1")));

        let mut session = Session::new();
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        };
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        let _ =
            process_lock_event(false, &mut session, &mock_operator, &mock_broker, "VIN", "zone")
                .await;

        assert!(
            mock_broker
                .set_bool_calls()
                .iter()
                .any(|(s, v)| s == SIGNAL_SESSION_ACTIVE && !*v),
            "should have set SessionActive to false"
        );
    }

    // TS-08-16: Verify manual StartSession works regardless of lock state.
    #[tokio::test]
    async fn test_manual_start_override() {
        let mock_broker = MockBrokerClient::new();
        let mock_operator = MockOperatorClient::new();
        mock_operator.on_start_return(Ok(make_start_response("sess-1")));

        let mut session = Session::new();

        let resp = process_manual_start(
            "zone-manual",
            &mut session,
            &mock_operator,
            &mock_broker,
            "DEMO-VIN-001",
        )
        .await;

        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert!(!resp.session_id.is_empty());
        assert!(session.is_active());
    }

    // TS-08-17: Verify manual StopSession works regardless of lock state.
    #[tokio::test]
    async fn test_manual_stop_override() {
        let mock_broker = MockBrokerClient::new();
        let mock_operator = MockOperatorClient::new();
        mock_operator.on_stop_return(Ok(make_stop_response("sess-1")));

        let mut session = Session::new();
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        };
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        let resp = process_manual_stop(&mut session, &mock_operator, &mock_broker).await;

        assert!(resp.is_ok());
        let resp = resp.unwrap();
        assert_eq!(resp.session_id, "sess-1");
        assert!(!session.is_active());
    }

    // TS-08-E1: Verify StartSession returns error when session already active.
    #[tokio::test]
    async fn test_start_session_already_active() {
        let mock_broker = MockBrokerClient::new();
        let mock_operator = MockOperatorClient::new();

        let mut session = Session::new();
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        };
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        let resp = process_manual_start(
            "zone-b",
            &mut session,
            &mock_operator,
            &mock_broker,
            "VIN",
        )
        .await;

        assert!(resp.is_err());
        match resp.unwrap_err() {
            ManualStartError::AlreadyExists(id) => assert_eq!(id, "sess-1"),
            other => panic!("expected AlreadyExists, got {other:?}"),
        }
    }

    // TS-08-E2: Verify StopSession returns error when no session active.
    #[tokio::test]
    async fn test_stop_session_no_active() {
        let mock_broker = MockBrokerClient::new();
        let mock_operator = MockOperatorClient::new();

        let mut session = Session::new();

        let resp = process_manual_stop(&mut session, &mock_operator, &mock_broker).await;

        assert!(resp.is_err());
        match resp.unwrap_err() {
            ManualStopError::NoActiveSession => {} // expected
            other => panic!("expected NoActiveSession, got {other:?}"),
        }
    }

    // TS-08-E6: Verify lock event during active session is a no-op.
    #[tokio::test]
    async fn test_lock_event_noop_when_active() {
        let mock_broker = MockBrokerClient::new();
        let mock_operator = MockOperatorClient::new();

        let mut session = Session::new();
        let rate = Rate {
            rate_type: "per_hour".to_string(),
            amount: 2.5,
            currency: "EUR".to_string(),
        };
        session.start("sess-1".to_string(), "zone-a".to_string(), 1_700_000_000, rate);

        let result = process_lock_event(
            true,
            &mut session,
            &mock_operator,
            &mock_broker,
            "VIN",
            "zone",
        )
        .await;
        assert!(result.is_ok());

        // Operator should NOT have been called.
        assert_eq!(mock_operator.start_call_count(), 0);

        // Session should still be active with original data.
        assert!(session.is_active());
        assert_eq!(session.status().unwrap().session_id, "sess-1");
    }

    // TS-08-E7: Verify unlock event without active session is a no-op.
    #[tokio::test]
    async fn test_unlock_event_noop_when_inactive() {
        let mock_broker = MockBrokerClient::new();
        let mock_operator = MockOperatorClient::new();

        let mut session = Session::new();

        let result = process_lock_event(
            false,
            &mut session,
            &mock_operator,
            &mock_broker,
            "VIN",
            "zone",
        )
        .await;
        assert!(result.is_ok());

        // Operator should NOT have been called.
        assert_eq!(mock_operator.stop_call_count(), 0);

        // Session should still be inactive.
        assert!(!session.is_active());
    }

    // TS-08-E9: Verify service continues after failing to publish SessionActive.
    #[tokio::test]
    async fn test_session_active_publish_failure() {
        let mock_broker = MockBrokerClient::new();
        mock_broker.fail_set_bool();
        let mock_operator = MockOperatorClient::new();
        mock_operator.on_start_return(Ok(make_start_response("sess-1")));

        let mut session = Session::new();

        let result = process_lock_event(
            true,
            &mut session,
            &mock_operator,
            &mock_broker,
            "VIN",
            "zone",
        )
        .await;

        // The operation should still succeed (or at least not panic).
        // Session state should be updated in memory even if publish failed.
        assert!(session.is_active(), "session should be active in memory despite publish failure");

        // Verify the service can still process subsequent events.
        mock_operator.on_stop_return(Ok(make_stop_response("sess-1")));
        let result2 = process_lock_event(
            false,
            &mut session,
            &mock_operator,
            &mock_broker,
            "VIN",
            "zone",
        )
        .await;
        // Should not panic — service continues despite broker failures.
        let _ = result;
        let _ = result2;
    }

    // TS-08-E11: Verify override resumes autonomous on next cycle.
    #[tokio::test]
    async fn test_override_resumes_autonomous() {
        let mock_broker = MockBrokerClient::new();
        let mock_operator = MockOperatorClient::new();

        // Manual start.
        mock_operator.on_start_return(Ok(make_start_response("sess-1")));
        let mut session = Session::new();
        let start_resp = process_manual_start(
            "zone-a",
            &mut session,
            &mock_operator,
            &mock_broker,
            "VIN",
        )
        .await;
        assert!(start_resp.is_ok());
        assert!(session.is_active());

        // Manual stop.
        mock_operator.on_stop_return(Ok(make_stop_response("sess-1")));
        let stop_resp = process_manual_stop(&mut session, &mock_operator, &mock_broker).await;
        assert!(stop_resp.is_ok());
        assert!(!session.is_active());

        // Autonomous lock event should start a new session.
        mock_operator.on_start_return(Ok(make_start_response("sess-2")));
        let result = process_lock_event(
            true,
            &mut session,
            &mock_operator,
            &mock_broker,
            "VIN",
            "zone",
        )
        .await;
        assert!(result.is_ok());
        assert!(session.is_active());
        assert_eq!(
            mock_operator.start_call_count(),
            2,
            "should have called start_session twice total"
        );
    }
}
