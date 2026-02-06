//! Session persistence storage.
//!
//! This module provides session storage for persisting session state
//! across container restarts.

use std::path::PathBuf;

use tokio::fs;
use tracing::{debug, error, info};

use crate::error::ParkingError;
use crate::session::Session;

/// Session store for persistence.
pub struct SessionStore {
    storage_path: PathBuf,
}

impl SessionStore {
    /// Create a new SessionStore.
    pub fn new(storage_path: PathBuf) -> Self {
        Self { storage_path }
    }

    /// Save a session to storage.
    pub async fn save(&self, session: &Session) -> Result<(), ParkingError> {
        debug!("Saving session to {:?}", self.storage_path);

        // Ensure parent directory exists
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                error!("Failed to create storage directory: {}", e);
                ParkingError::StorageError(format!("Failed to create directory: {}", e))
            })?;
        }

        let json = serde_json::to_string_pretty(session).map_err(|e| {
            error!("Failed to serialize session: {}", e);
            ParkingError::StorageError(format!("Serialization failed: {}", e))
        })?;

        fs::write(&self.storage_path, json).await.map_err(|e| {
            error!("Failed to write session file: {}", e);
            ParkingError::StorageError(format!("Failed to write file: {}", e))
        })?;

        info!("Session saved to {:?}", self.storage_path);
        Ok(())
    }

    /// Load a session from storage.
    pub async fn load(&self) -> Result<Option<Session>, ParkingError> {
        debug!("Loading session from {:?}", self.storage_path);

        if !self.storage_path.exists() {
            debug!("No session file found");
            return Ok(None);
        }

        let json = fs::read_to_string(&self.storage_path).await.map_err(|e| {
            error!("Failed to read session file: {}", e);
            ParkingError::StorageError(format!("Failed to read file: {}", e))
        })?;

        let session: Session = serde_json::from_str(&json).map_err(|e| {
            error!("Failed to deserialize session: {}", e);
            ParkingError::StorageError(format!("Deserialization failed: {}", e))
        })?;

        info!("Session loaded from {:?}", self.storage_path);
        Ok(Some(session))
    }

    /// Clear the stored session.
    pub async fn clear(&self) -> Result<(), ParkingError> {
        debug!("Clearing session from {:?}", self.storage_path);

        if self.storage_path.exists() {
            fs::remove_file(&self.storage_path).await.map_err(|e| {
                error!("Failed to delete session file: {}", e);
                ParkingError::StorageError(format!("Failed to delete file: {}", e))
            })?;
            info!("Session file cleared");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::Location;
    use crate::session::SessionState;
    use proptest::prelude::*;
    use tempfile::TempDir;

    fn create_test_session(
        session_id: &str,
        zone_id: &str,
        lat: f64,
        lng: f64,
    ) -> Session {
        let mut session = Session::new_starting(Location::new(lat, lng), zone_id.to_string());
        session.session_id = session_id.to_string();
        session.state = SessionState::Active;
        session.hourly_rate = 2.50;
        session.current_cost = 1.25;
        session
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("session.json");
        let store = SessionStore::new(storage_path);

        let session = create_test_session("session-123", "zone-1", 37.7749, -122.4194);

        // Save
        store.save(&session).await.unwrap();

        // Load
        let loaded = store.load().await.unwrap().unwrap();
        assert_eq!(loaded.session_id, "session-123");
        assert_eq!(loaded.zone_id, "zone-1");
        assert_eq!(loaded.state, SessionState::Active);
    }

    #[tokio::test]
    async fn test_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("nonexistent.json");
        let store = SessionStore::new(storage_path);

        let result = store.load().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_clear() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("session.json");
        let store = SessionStore::new(storage_path.clone());

        let session = create_test_session("session-123", "zone-1", 37.7749, -122.4194);
        store.save(&session).await.unwrap();
        assert!(storage_path.exists());

        store.clear().await.unwrap();
        assert!(!storage_path.exists());
    }

    // Property 13: Session Persistence Round-Trip
    // Validates: Requirements 7.3
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_session_persistence_round_trip(
            session_id in "[a-z0-9-]{8,36}",
            zone_id in "[a-z0-9-]{4,20}",
            lat in -90.0f64..90.0,
            lng in -180.0f64..180.0,
            hourly_rate in 0.5f64..50.0,
            current_cost in 0.0f64..1000.0
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let storage_path = temp_dir.path().join("session.json");
                let store = SessionStore::new(storage_path);

                let mut original = Session::new_starting(
                    Location::new(lat, lng),
                    zone_id.clone(),
                );
                original.session_id = session_id.clone();
                original.state = SessionState::Active;
                original.hourly_rate = hourly_rate;
                original.current_cost = current_cost;

                // Save and load
                store.save(&original).await.unwrap();
                let loaded = store.load().await.unwrap().unwrap();

                // Verify round-trip preserves all fields
                prop_assert_eq!(loaded.session_id, original.session_id);
                prop_assert_eq!(loaded.zone_id, original.zone_id);
                prop_assert_eq!(loaded.state, original.state);
                prop_assert!((loaded.location.latitude - original.location.latitude).abs() < 0.0001);
                prop_assert!((loaded.location.longitude - original.location.longitude).abs() < 0.0001);
                prop_assert!((loaded.hourly_rate - original.hourly_rate).abs() < 0.01);
                prop_assert!((loaded.current_cost - original.current_cost).abs() < 0.01);
                Ok(())
            })?;
        }
    }
}
