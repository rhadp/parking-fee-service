//! Session manager for the PARKING_OPERATOR_ADAPTOR.
//!
//! Maintains the current active session state and supports both autonomous
//! (lock/unlock event-driven) and manual (gRPC override) session management.

use std::sync::Arc;
use tokio::sync::Mutex;

/// An active parking session.
#[derive(Debug, Clone)]
pub struct ActiveSession {
    /// Session identifier from the PARKING_OPERATOR.
    pub session_id: String,
    /// Zone identifier for this session.
    pub zone_id: String,
    /// Start time as Unix timestamp.
    pub start_time: i64,
}

/// Manages parking session state.
///
/// Provides thread-safe session lifecycle management via `Arc<Mutex<...>>`.
/// Supports both autonomous (lock/unlock events) and manual (gRPC override)
/// session operations.
#[derive(Debug)]
pub struct SessionManager {
    /// The currently active session, if any.
    active_session: Option<ActiveSession>,
    /// Whether the current session was started by a manual override.
    override_active: bool,
}

impl SessionManager {
    /// Create a new session manager with no active session.
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(SessionManager {
            active_session: None,
            override_active: false,
        }))
    }

    /// Start a new session.
    ///
    /// Returns `Ok(())` if the session was started, or `Err` with a message
    /// if a session is already active.
    pub fn start_session(
        &mut self,
        session_id: String,
        zone_id: String,
        start_time: i64,
        is_override: bool,
    ) -> Result<(), String> {
        if self.active_session.is_some() {
            return Err("a session is already active".to_string());
        }

        self.active_session = Some(ActiveSession {
            session_id,
            zone_id,
            start_time,
        });
        self.override_active = is_override;
        Ok(())
    }

    /// Stop the currently active session.
    ///
    /// Returns `Ok(session_id)` if the session was stopped, or `Err` with a
    /// message if the provided session_id does not match the active session.
    pub fn stop_session(&mut self, session_id: &str) -> Result<String, String> {
        match &self.active_session {
            Some(session) if session.session_id == session_id => {
                let id = session.session_id.clone();
                self.active_session = None;
                self.override_active = false;
                Ok(id)
            }
            Some(session) => Err(format!(
                "session_id mismatch: active={}, requested={}",
                session.session_id, session_id
            )),
            None => Err(format!("no active session with id {}", session_id)),
        }
    }

    /// Check if there is an active session.
    pub fn has_active_session(&self) -> bool {
        self.active_session.is_some()
    }

    /// Get the current session ID, if any.
    pub fn current_session_id(&self) -> Option<&str> {
        self.active_session.as_ref().map(|s| s.session_id.as_str())
    }

    /// Get the active session details, if any.
    pub fn active_session(&self) -> Option<&ActiveSession> {
        self.active_session.as_ref()
    }

    /// Check if the current session was started by a manual override.
    pub fn is_override(&self) -> bool {
        self.override_active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_session_manager_has_no_session() {
        let mgr = SessionManager::new();
        let lock = mgr.lock().await;
        assert!(!lock.has_active_session());
        assert!(lock.current_session_id().is_none());
        assert!(lock.active_session().is_none());
    }

    #[tokio::test]
    async fn test_start_session() {
        let mgr = SessionManager::new();
        let mut lock = mgr.lock().await;
        let result =
            lock.start_session("sess-1".into(), "zone-a".into(), 1000, false);
        assert!(result.is_ok());
        assert!(lock.has_active_session());
        assert_eq!(lock.current_session_id(), Some("sess-1"));
        assert!(!lock.is_override());
    }

    #[tokio::test]
    async fn test_start_session_already_active_returns_error() {
        let mgr = SessionManager::new();
        let mut lock = mgr.lock().await;
        lock.start_session("sess-1".into(), "zone-a".into(), 1000, false)
            .unwrap();

        let result =
            lock.start_session("sess-2".into(), "zone-b".into(), 2000, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already active"));
    }

    #[tokio::test]
    async fn test_stop_session() {
        let mgr = SessionManager::new();
        let mut lock = mgr.lock().await;
        lock.start_session("sess-1".into(), "zone-a".into(), 1000, false)
            .unwrap();

        let result = lock.stop_session("sess-1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "sess-1");
        assert!(!lock.has_active_session());
    }

    #[tokio::test]
    async fn test_stop_session_wrong_id() {
        let mgr = SessionManager::new();
        let mut lock = mgr.lock().await;
        lock.start_session("sess-1".into(), "zone-a".into(), 1000, false)
            .unwrap();

        let result = lock.stop_session("sess-wrong");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("mismatch"));
        // Session should still be active
        assert!(lock.has_active_session());
    }

    #[tokio::test]
    async fn test_stop_session_no_active() {
        let mgr = SessionManager::new();
        let mut lock = mgr.lock().await;

        let result = lock.stop_session("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no active session"));
    }

    #[tokio::test]
    async fn test_override_flag() {
        let mgr = SessionManager::new();
        let mut lock = mgr.lock().await;
        lock.start_session("sess-1".into(), "zone-a".into(), 1000, true)
            .unwrap();
        assert!(lock.is_override());

        lock.stop_session("sess-1").unwrap();
        assert!(!lock.is_override());
    }

    #[tokio::test]
    async fn test_active_session_details() {
        let mgr = SessionManager::new();
        let mut lock = mgr.lock().await;
        lock.start_session("sess-1".into(), "zone-a".into(), 1234567890, false)
            .unwrap();

        let session = lock.active_session().unwrap();
        assert_eq!(session.session_id, "sess-1");
        assert_eq!(session.zone_id, "zone-a");
        assert_eq!(session.start_time, 1234567890);
    }
}
