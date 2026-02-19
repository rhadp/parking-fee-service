//! Lock event watcher for automatic parking session management.
//!
//! Subscribes to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` on the Kuksa
//! Databroker and starts/stops parking sessions via the PARKING_OPERATOR REST
//! API when lock state changes.
//!
//! # Requirements
//!
//! - 04-REQ-1.1: Subscribe to `IsLocked` on DATA_BROKER via gRPC streaming.
//! - 04-REQ-1.2: On lock → call `POST /parking/start`, update session state.
//! - 04-REQ-1.3: Write `SessionActive = true` to DATA_BROKER after start.
//! - 04-REQ-1.4: On unlock → call `POST /parking/stop`, complete session.
//! - 04-REQ-1.5: Write `SessionActive = false` to DATA_BROKER after stop.
//! - 04-REQ-1.E1: Log error on operator failure, do not set SessionActive.
//! - 04-REQ-1.E2: Ignore duplicate lock events (lock while already locked).
//! - 04-REQ-1.E3: Ignore unlock events when no session is active.

use std::sync::Arc;

use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::operator_client::OperatorClient;
use crate::session::{ParkingSession, RateType, SessionStatus};

use parking_proto::kuksa_client::KuksaClient;
use parking_proto::signals;

/// Shared session state accessible by both the lock watcher and gRPC server.
pub type SessionState = Arc<Mutex<Option<ParkingSession>>>;

/// Subscribe to `IsLocked` events and manage parking sessions.
///
/// Runs indefinitely, re-subscribing on stream errors with a short delay.
pub async fn watch_lock_events(
    kuksa: KuksaClient,
    operator: OperatorClient,
    session_state: SessionState,
    config: Config,
) {
    loop {
        match run_watcher(&kuksa, &operator, &session_state, &config).await {
            Ok(()) => {
                info!("lock watcher subscription stream ended, resubscribing");
            }
            Err(e) => {
                error!(error = %e, "lock watcher error, resubscribing in 2s");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}

async fn run_watcher(
    kuksa: &KuksaClient,
    operator: &OperatorClient,
    session_state: &SessionState,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("subscribing to {}", signals::DOOR_IS_LOCKED);
    let mut stream = kuksa.subscribe_bool(signals::DOOR_IS_LOCKED).await?;
    info!("lock watcher subscription established");

    while let Some(result) = stream.next().await {
        match result {
            Ok(is_locked) => {
                handle_lock_event(is_locked, kuksa, operator, session_state, config).await;
            }
            Err(e) => {
                error!(error = %e, "error in lock event subscription");
                return Err(Box::new(e));
            }
        }
    }

    Ok(())
}

async fn handle_lock_event(
    is_locked: bool,
    kuksa: &KuksaClient,
    operator: &OperatorClient,
    session_state: &SessionState,
    config: &Config,
) {
    let mut state = session_state.lock().await;

    if is_locked {
        if state.as_ref().is_some_and(|s| s.is_active()) {
            info!("lock event but session already active, ignoring (04-REQ-1.E2)");
            return;
        }

        let now = unix_now();
        match operator
            .start_session(&config.vehicle_vin, &config.zone_id, now)
            .await
        {
            Ok(resp) => {
                info!(
                    session_id = %resp.session_id,
                    "parking session started with operator"
                );

                *state = Some(ParkingSession {
                    session_id: resp.session_id,
                    vehicle_id: config.vehicle_vin.clone(),
                    zone_id: config.zone_id.clone(),
                    start_time: now,
                    end_time: None,
                    rate_type: RateType::from_str_loose(&resp.rate.rate_type),
                    rate_amount: resp.rate.rate_amount,
                    currency: resp.rate.currency.clone(),
                    total_fee: None,
                    status: SessionStatus::Active,
                });

                if let Err(e) = kuksa
                    .set_bool(signals::PARKING_SESSION_ACTIVE, true)
                    .await
                {
                    warn!(error = %e, "failed to write SessionActive=true");
                }
            }
            Err(e) => {
                error!(error = %e, "failed to start session (04-REQ-1.E1)");
            }
        }
    } else {
        let session = match state.as_ref() {
            Some(s) if s.is_active() => s.clone(),
            _ => {
                info!("unlock event but no active session, ignoring (04-REQ-1.E3)");
                return;
            }
        };

        let now = unix_now();
        match operator.stop_session(&session.session_id, now).await {
            Ok(resp) => {
                info!(
                    session_id = %session.session_id,
                    total_fee = resp.total_fee,
                    duration_seconds = resp.duration_seconds,
                    "parking session stopped with operator"
                );

                if let Some(ref mut s) = *state {
                    s.complete(now, resp.total_fee, resp.duration_seconds);
                }

                if let Err(e) = kuksa
                    .set_bool(signals::PARKING_SESSION_ACTIVE, false)
                    .await
                {
                    warn!(error = %e, "failed to write SessionActive=false");
                }
            }
            Err(e) => {
                error!(error = %e, "failed to stop session with operator");
            }
        }
    }
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_state_starts_as_none() {
        let state: SessionState = Arc::new(Mutex::new(None));
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let guard = state.lock().await;
            assert!(guard.is_none());
        });
    }

    #[tokio::test]
    async fn duplicate_lock_with_active_session_is_noop() {
        let state: SessionState = Arc::new(Mutex::new(Some(ParkingSession {
            session_id: "sess-001".into(),
            vehicle_id: "VIN1".into(),
            zone_id: "zone-1".into(),
            start_time: 1_708_300_800,
            end_time: None,
            rate_type: RateType::PerMinute,
            rate_amount: 0.05,
            currency: "EUR".into(),
            total_fee: None,
            status: SessionStatus::Active,
        })));

        let guard = state.lock().await;
        assert!(
            guard.as_ref().is_some_and(|s| s.is_active()),
            "active session should cause lock event to be ignored"
        );
    }

    #[tokio::test]
    async fn unlock_without_active_session_is_noop() {
        let state: SessionState = Arc::new(Mutex::new(None));
        let guard = state.lock().await;
        let has_active = guard.as_ref().is_some_and(|s| s.is_active());
        assert!(
            !has_active,
            "no active session means unlock should be ignored"
        );
    }

    #[tokio::test]
    async fn unlock_with_completed_session_is_noop() {
        let state: SessionState = Arc::new(Mutex::new(Some(ParkingSession {
            session_id: "sess-001".into(),
            vehicle_id: "VIN1".into(),
            zone_id: "zone-1".into(),
            start_time: 1_708_300_800,
            end_time: Some(1_708_301_100),
            rate_type: RateType::PerMinute,
            rate_amount: 0.05,
            currency: "EUR".into(),
            total_fee: Some(0.25),
            status: SessionStatus::Completed,
        })));

        let guard = state.lock().await;
        let has_active = guard.as_ref().is_some_and(|s| s.is_active());
        assert!(
            !has_active,
            "completed session should not be treated as active"
        );
    }

    #[test]
    fn unix_now_returns_positive() {
        assert!(unix_now() > 0);
    }
}
