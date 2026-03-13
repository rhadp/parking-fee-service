//! Autonomous session management module.
//!
//! Subscribes to lock/unlock events from DATA_BROKER and manages parking
//! sessions automatically via the PARKING_OPERATOR REST API.

use crate::broker::subscriber::BrokerSubscriber;
use crate::broker::SessionPublisher;
use crate::operator::OperatorApi;
use crate::session::{SessionManager, SessionState};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

/// Run the autonomous event loop that subscribes to lock/unlock events.
///
/// This function connects to DATA_BROKER, subscribes to the IsLocked signal,
/// and triggers session start/stop based on lock/unlock events.
pub async fn run_autonomous_loop(
    broker_addr: String,
    session: Arc<Mutex<SessionManager>>,
    operator: Arc<dyn OperatorApi>,
    publisher: Arc<dyn SessionPublisher>,
    vehicle_id: String,
    zone_id: String,
) {
    // Connect to DATA_BROKER with retry
    let mut subscriber = match BrokerSubscriber::connect(&broker_addr).await {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "failed to connect BrokerSubscriber; autonomous mode disabled");
            return;
        }
    };

    // Subscribe to lock events
    let mut stream = match subscriber.subscribe_lock_events().await {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "failed to subscribe to lock events; autonomous mode disabled");
            return;
        }
    };

    info!("autonomous event loop started; listening for lock/unlock events");

    // Process lock/unlock events
    use tokio_stream::StreamExt;
    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                if let Some(is_locked) = BrokerSubscriber::extract_is_locked(&response) {
                    if is_locked {
                        handle_lock_event(
                            &session,
                            operator.as_ref(),
                            publisher.as_ref(),
                            &vehicle_id,
                            &zone_id,
                        )
                        .await;
                    } else {
                        handle_unlock_event(
                            &session,
                            operator.as_ref(),
                            publisher.as_ref(),
                        )
                        .await;
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "error receiving lock event from DATA_BROKER");
            }
        }
    }

    warn!("DATA_BROKER subscription stream ended; autonomous mode inactive");
}

/// Handle a lock event: start a parking session if idle.
pub async fn handle_lock_event(
    session: &Arc<Mutex<SessionManager>>,
    operator: &dyn OperatorApi,
    publisher: &dyn SessionPublisher,
    vehicle_id: &str,
    zone_id: &str,
) {
    let mut s = session.lock().await;

    // Only start if currently idle
    if *s.state() != SessionState::Idle {
        info!(state = ?s.state(), "lock event ignored; session not idle");
        return;
    }

    if let Err(e) = s.try_start() {
        info!(error = %e, "lock event ignored; cannot start session");
        return;
    }

    // Release lock before making the HTTP call
    drop(s);

    match operator.start_session(vehicle_id, zone_id).await {
        Ok(resp) => {
            info!(session_id = %resp.session_id, "autonomous session started");
            let mut s = session.lock().await;
            s.confirm_start(resp.session_id);

            // Publish SessionActive = true to DATA_BROKER
            if let Err(e) = publisher.set_session_active(true).await {
                error!(error = %e, "failed to publish SessionActive=true");
            }
        }
        Err(e) => {
            error!(error = %e, "autonomous start_session failed");
            let mut s = session.lock().await;
            s.fail_start();
        }
    }
}

/// Handle an unlock event: stop a parking session if active.
pub async fn handle_unlock_event(
    session: &Arc<Mutex<SessionManager>>,
    operator: &dyn OperatorApi,
    publisher: &dyn SessionPublisher,
) {
    let mut s = session.lock().await;

    // Only stop if currently active
    if *s.state() != SessionState::Active {
        info!(state = ?s.state(), "unlock event ignored; session not active");
        return;
    }

    if let Err(e) = s.try_stop() {
        info!(error = %e, "unlock event ignored; cannot stop session");
        return;
    }

    let session_id = s.session_id().unwrap_or_default().to_string();

    // Release lock before making the HTTP call
    drop(s);

    match operator.stop_session(&session_id).await {
        Ok(resp) => {
            info!(
                session_id = %resp.session_id,
                duration = resp.duration,
                fee = resp.fee,
                "autonomous session stopped"
            );
            let mut s = session.lock().await;
            s.confirm_stop();

            // Publish SessionActive = false to DATA_BROKER
            if let Err(e) = publisher.set_session_active(false).await {
                error!(error = %e, "failed to publish SessionActive=false");
            }
        }
        Err(e) => {
            error!(error = %e, "autonomous stop_session failed");
            let mut s = session.lock().await;
            s.fail_stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Rate;
    use crate::testing::{MockBrokerPublisher, MockOperatorClient};

    fn make_session() -> Arc<Mutex<SessionManager>> {
        Arc::new(Mutex::new(SessionManager::new(Some(
            "zone-demo-1".to_string(),
        ))))
    }

    // -----------------------------------------------------------------------
    // TS-08-1: Autonomous Start on Lock
    // -----------------------------------------------------------------------

    /// TS-08-1: When IsLocked changes to true and no session is active,
    /// the adaptor calls the operator start API.
    #[tokio::test]
    async fn test_autonomous_start_on_lock() {
        let session = make_session();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerPublisher::new();

        handle_lock_event(&session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1").await;

        assert_eq!(operator.start_count(), 1, "start_session should be called once");
        let calls = operator.start_calls.lock().unwrap();
        assert_eq!(calls[0].0, "DEMO-VIN-001");
        assert_eq!(calls[0].1, "zone-demo-1");

        let s = session.lock().await;
        assert!(s.is_active(), "session should be active after lock event");
    }

    // -----------------------------------------------------------------------
    // TS-08-3: SessionActive Written on Start
    // -----------------------------------------------------------------------

    /// TS-08-3: After starting a session, SessionActive=true is written to DATA_BROKER.
    #[tokio::test]
    async fn test_session_active_written_on_start() {
        let session = make_session();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerPublisher::new();

        handle_lock_event(&session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1").await;

        assert!(broker.was_called_with(true), "broker should have set_session_active(true)");
    }

    // -----------------------------------------------------------------------
    // TS-08-4: Autonomous Stop on Unlock
    // -----------------------------------------------------------------------

    /// TS-08-4: When IsLocked changes to false and a session is active,
    /// the adaptor calls operator stop API.
    #[tokio::test]
    async fn test_autonomous_stop_on_unlock() {
        let session = make_session();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerPublisher::new();

        // First start a session
        handle_lock_event(&session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1").await;
        let session_id = {
            let s = session.lock().await;
            s.session_id().unwrap().to_string()
        };

        operator.reset();
        broker.reset();

        // Now unlock
        handle_unlock_event(&session, &operator, &broker).await;

        assert_eq!(operator.stop_count(), 1, "stop_session should be called once");
        let calls = operator.stop_calls.lock().unwrap();
        assert_eq!(calls[0], session_id);
    }

    // -----------------------------------------------------------------------
    // TS-08-5: Session Cleared After Stop
    // -----------------------------------------------------------------------

    /// TS-08-5: After successful stop, session state is cleared.
    #[tokio::test]
    async fn test_session_cleared_after_stop_autonomous() {
        let session = make_session();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerPublisher::new();

        handle_lock_event(&session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1").await;
        handle_unlock_event(&session, &operator, &broker).await;

        let s = session.lock().await;
        assert!(!s.is_active(), "session should be inactive after unlock");
    }

    // -----------------------------------------------------------------------
    // TS-08-6: SessionActive Written on Stop
    // -----------------------------------------------------------------------

    /// TS-08-6: After stopping a session, SessionActive=false is written to DATA_BROKER.
    #[tokio::test]
    async fn test_session_active_written_on_stop() {
        let session = make_session();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerPublisher::new();

        handle_lock_event(&session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1").await;
        broker.reset();

        handle_unlock_event(&session, &operator, &broker).await;

        assert!(
            broker.was_called_with(false),
            "broker should have set_session_active(false)"
        );
    }

    // -----------------------------------------------------------------------
    // TS-08-9: Resume Autonomous After Override
    // -----------------------------------------------------------------------

    /// TS-08-9: After manual stop, autonomous behavior resumes on next lock event.
    #[tokio::test]
    async fn test_resume_autonomous_after_override() {
        let session = make_session();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerPublisher::new();

        // Start manually (simulate gRPC override)
        {
            let mut s = session.lock().await;
            s.start(
                "manual-sess",
                "zone-demo-1",
                Rate {
                    rate_type: "per_hour".to_string(),
                    amount: 2.50,
                    currency: "EUR".to_string(),
                },
            )
            .unwrap();
        }

        // Stop manually
        {
            let mut s = session.lock().await;
            s.stop().unwrap();
        }

        // Autonomous lock event should start a new session
        handle_lock_event(&session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1").await;

        let s = session.lock().await;
        assert!(s.is_active(), "session should be active after autonomous lock");
        assert_eq!(operator.start_count(), 1);
    }

    // -----------------------------------------------------------------------
    // TS-08-E1: Lock While Session Active
    // -----------------------------------------------------------------------

    /// TS-08-E1: Lock event while session active is a no-op.
    #[tokio::test]
    async fn test_lock_while_session_active() {
        let session = make_session();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerPublisher::new();

        // Start a session
        handle_lock_event(&session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1").await;
        let first_session_id = {
            let s = session.lock().await;
            s.session_id().unwrap().to_string()
        };

        operator.reset();
        broker.reset();

        // Second lock event should be ignored
        handle_lock_event(&session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1").await;

        assert_eq!(operator.start_count(), 0, "start_session should NOT be called again");
        let s = session.lock().await;
        assert_eq!(
            s.session_id().unwrap(),
            first_session_id,
            "session_id should not change"
        );
    }

    // -----------------------------------------------------------------------
    // TS-08-E2: Operator Start Failure (with retry)
    // -----------------------------------------------------------------------

    /// TS-08-E2: Start failure after 3 retries logs error, no session created.
    #[tokio::test]
    async fn test_operator_start_failure() {
        let session = make_session();
        let operator = MockOperatorClient::new();
        operator.set_start_should_fail(true);
        let broker = MockBrokerPublisher::new();

        // Wrap in retry client
        let retry_operator = crate::operator::RetryOperatorClient::new(operator);

        // Use tokio::time::pause to avoid real delays
        tokio::time::pause();

        handle_lock_event(
            &session,
            &retry_operator,
            &broker,
            "DEMO-VIN-001",
            "zone-demo-1",
        )
        .await;

        // The retry wrapper calls the inner mock 3 times
        assert_eq!(
            retry_operator.inner.start_count(),
            3,
            "should have attempted 3 times"
        );
        let s = session.lock().await;
        assert!(!s.is_active(), "session should NOT be active after failure");
    }

    // -----------------------------------------------------------------------
    // TS-08-E3: Unlock While No Session
    // -----------------------------------------------------------------------

    /// TS-08-E3: Unlock event while no session is a no-op.
    #[tokio::test]
    async fn test_unlock_while_no_session() {
        let session = make_session();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerPublisher::new();

        handle_unlock_event(&session, &operator, &broker).await;

        assert_eq!(operator.stop_count(), 0, "stop_session should NOT be called");
    }

    // -----------------------------------------------------------------------
    // TS-08-E4: Operator Stop Failure (with retry)
    // -----------------------------------------------------------------------

    /// TS-08-E4: Stop failure after 3 retries logs error, session not cleared.
    #[tokio::test]
    async fn test_operator_stop_failure() {
        let session = make_session();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerPublisher::new();

        // Start a session first
        handle_lock_event(&session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1").await;

        // Now set stop to fail and wrap in retry
        let fail_operator = MockOperatorClient::new();
        fail_operator.set_stop_should_fail(true);
        let retry_operator = crate::operator::RetryOperatorClient::new(fail_operator);

        tokio::time::pause();

        handle_unlock_event(&session, &retry_operator, &broker).await;

        assert_eq!(
            retry_operator.inner.stop_count(),
            3,
            "should have attempted 3 times"
        );
        let s = session.lock().await;
        assert!(s.is_active(), "session should remain active after stop failure");
    }

    // -----------------------------------------------------------------------
    // TS-08-E9: SessionActive Write Failure
    // -----------------------------------------------------------------------

    /// TS-08-E9: SessionActive write failure is logged but session continues.
    #[tokio::test]
    async fn test_session_active_write_failure() {
        let session = make_session();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerPublisher::new();
        broker.set_should_fail(true);

        handle_lock_event(&session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1").await;

        let s = session.lock().await;
        assert!(
            s.is_active(),
            "session should be active even when broker write fails"
        );
    }

    // -----------------------------------------------------------------------
    // TS-08-14: SessionActive Written (start + stop cycle)
    // -----------------------------------------------------------------------

    /// TS-08-14: Session start/stop writes SessionActive to DATA_BROKER.
    #[tokio::test]
    async fn test_session_active_written_cycle() {
        let session = make_session();
        let operator = MockOperatorClient::new();
        let broker = MockBrokerPublisher::new();

        handle_lock_event(&session, &operator, &broker, "DEMO-VIN-001", "zone-demo-1").await;
        handle_unlock_event(&session, &operator, &broker).await;

        let calls = broker.get_set_calls();
        assert_eq!(calls.len(), 2, "should have 2 set_session_active calls");
        assert!(calls[0], "first call should be true (start)");
        assert!(!calls[1], "second call should be false (stop)");
    }

    // -----------------------------------------------------------------------
    // TS-08-13: Lock Subscription (verified by autonomous loop structure)
    // -----------------------------------------------------------------------

    /// TS-08-13: On startup, the adaptor subscribes to IsLocked signal.
    /// This is structurally verified: run_autonomous_loop calls
    /// subscriber.subscribe_lock_events() as its first action.
    /// We verify the subscriber constant matches the expected signal path.
    #[test]
    fn test_lock_subscription_signal_path() {
        assert_eq!(
            crate::broker::subscriber::IS_LOCKED_SIGNAL,
            "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked"
        );
    }
}

/// Property-based tests for autonomous session management.
#[cfg(test)]
mod proptest_tests {
    use super::*;
    use crate::session::Rate;
    use crate::testing::{MockBrokerPublisher, MockOperatorClient};
    use proptest::prelude::*;

    /// Helper: create a tokio runtime for property tests.
    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    fn make_session() -> Arc<Mutex<SessionManager>> {
        Arc::new(Mutex::new(SessionManager::new(Some(
            "zone-demo-1".to_string(),
        ))))
    }

    // -----------------------------------------------------------------------
    // TS-08-P1: Autonomous Start on Lock
    // -----------------------------------------------------------------------

    proptest! {
        /// TS-08-P1: For any lock event when no session is active, start
        /// is called and state is updated.
        #[test]
        fn proptest_autonomous_start_on_lock(
            zone_id in "[a-z]{3,10}",
            vehicle_id in "[A-Z]{3}-[0-9]{3}",
        ) {
            let rt = rt();
            rt.block_on(async {
                let session = make_session();
                let operator = MockOperatorClient::new();
                let broker = MockBrokerPublisher::new();

                prop_assert!(!session.lock().await.is_active());

                handle_lock_event(&session, &operator, &broker, &vehicle_id, &zone_id).await;

                prop_assert!(session.lock().await.is_active());
                prop_assert!(broker.was_called_with(true));
                prop_assert_eq!(operator.start_count(), 1);

                Ok(())
            })?;
        }
    }

    // -----------------------------------------------------------------------
    // TS-08-P2: Autonomous Stop on Unlock
    // -----------------------------------------------------------------------

    proptest! {
        /// TS-08-P2: For any unlock event when session is active, stop
        /// is called and state is cleared.
        #[test]
        fn proptest_autonomous_stop_on_unlock(
            session_id in "[a-z0-9]{5,15}",
        ) {
            let rt = rt();
            rt.block_on(async {
                let session = make_session();
                let rate = Rate {
                    rate_type: "per_hour".to_string(),
                    amount: 2.50,
                    currency: "EUR".to_string(),
                };
                {
                    let mut s = session.lock().await;
                    s.start(&session_id, "zone-demo-1", rate).unwrap();
                }

                let operator = MockOperatorClient::new();
                let broker = MockBrokerPublisher::new();

                handle_unlock_event(&session, &operator, &broker).await;

                prop_assert!(!session.lock().await.is_active());
                prop_assert!(broker.was_called_with(false));

                Ok(())
            })?;
        }
    }

    // -----------------------------------------------------------------------
    // TS-08-P3: Session Idempotency
    // -----------------------------------------------------------------------

    proptest! {
        /// TS-08-P3: Lock while active or unlock while inactive = no operator calls.
        #[test]
        fn proptest_session_idempotency(
            events in proptest::collection::vec(proptest::bool::ANY, 2..10),
        ) {
            let rt = rt();
            rt.block_on(async {
                let session = make_session();
                let operator = MockOperatorClient::new();
                let broker = MockBrokerPublisher::new();

                for event in &events {
                    let was_active = session.lock().await.is_active();
                    let start_before = operator.start_count();
                    let stop_before = operator.stop_count();

                    if *event {
                        handle_lock_event(
                            &session, &operator, &broker,
                            "VIN-001", "zone-1",
                        ).await;
                    } else {
                        handle_unlock_event(&session, &operator, &broker).await;
                    }

                    // If lock event while active, no new start call
                    if *event && was_active {
                        prop_assert_eq!(
                            operator.start_count(), start_before,
                            "lock while active should not call start"
                        );
                    }
                    // If unlock event while inactive, no stop call
                    if !*event && !was_active {
                        prop_assert_eq!(
                            operator.stop_count(), stop_before,
                            "unlock while inactive should not call stop"
                        );
                    }
                }
                Ok(())
            })?;
        }
    }

    // -----------------------------------------------------------------------
    // TS-08-P4: Manual Override Consistency
    // -----------------------------------------------------------------------

    proptest! {
        /// TS-08-P4: After manual override, next lock/unlock cycle works autonomously.
        #[test]
        fn proptest_manual_override_consistency(
            zone_id in "[a-z]{3,8}",
        ) {
            let rt = rt();
            rt.block_on(async {
                let session = make_session();
                let operator = MockOperatorClient::new();
                let broker = MockBrokerPublisher::new();

                // Manual start
                {
                    let mut s = session.lock().await;
                    s.start(
                        "manual-sess",
                        &zone_id,
                        Rate {
                            rate_type: "per_hour".to_string(),
                            amount: 2.50,
                            currency: "EUR".to_string(),
                        },
                    ).unwrap();
                }

                // Manual stop
                {
                    let mut s = session.lock().await;
                    s.stop().unwrap();
                }

                // Autonomous lock should work
                handle_lock_event(&session, &operator, &broker, "VIN-001", &zone_id).await;
                prop_assert!(session.lock().await.is_active());

                // Autonomous unlock should work
                handle_unlock_event(&session, &operator, &broker).await;
                prop_assert!(!session.lock().await.is_active());

                Ok(())
            })?;
        }
    }

    // -----------------------------------------------------------------------
    // TS-08-P5: Operator Retry Logic
    // -----------------------------------------------------------------------

    proptest! {
        /// TS-08-P5: Failures always result in exactly 3 retry attempts.
        #[test]
        fn proptest_operator_retry_logic(
            do_start in proptest::bool::ANY,
        ) {
            let rt = rt();
            rt.block_on(async {
                tokio::time::pause();

                let session = make_session();
                let mock_op = MockOperatorClient::new();
                let broker = MockBrokerPublisher::new();

                if do_start {
                    mock_op.set_start_should_fail(true);
                    let retry_op = crate::operator::RetryOperatorClient::new(mock_op);
                    handle_lock_event(
                        &session, &retry_op, &broker,
                        "VIN-001", "zone-1",
                    ).await;
                    prop_assert_eq!(retry_op.inner.start_count(), 3);
                    prop_assert!(!session.lock().await.is_active());
                } else {
                    // Start a session first with a working operator
                    let good_op = MockOperatorClient::new();
                    handle_lock_event(
                        &session, &good_op, &broker,
                        "VIN-001", "zone-1",
                    ).await;

                    mock_op.set_stop_should_fail(true);
                    let retry_op = crate::operator::RetryOperatorClient::new(mock_op);
                    handle_unlock_event(&session, &retry_op, &broker).await;
                    prop_assert_eq!(retry_op.inner.stop_count(), 3);
                    prop_assert!(session.lock().await.is_active());
                }
                Ok(())
            })?;
        }
    }
}
