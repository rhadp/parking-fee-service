//! Offload timer management for adapter containers.
//!
//! When a parking session ends, an offload timer is started. If no new
//! session starts before the timer expires, the adapter container is
//! automatically stopped and removed.
//!
//! # Requirements
//!
//! - 04-REQ-5.1: Start offloading timer when a session ends.
//! - 04-REQ-5.2: Offload adapter when timer expires.
//! - 04-REQ-5.3: Cancel timer when a new session starts.
//! - 04-REQ-5.4: Configurable offloading timeout.
//! - 04-REQ-5.E1: Cancel timer on manual removal.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// OffloadCallback — what happens when a timer fires
// ---------------------------------------------------------------------------

/// Callback invoked when an offload timer expires.
///
/// The callback receives the adapter_id and must perform the actual
/// container stop+remove and state transition.
pub type OffloadCallback = Arc<dyn Fn(String) + Send + Sync>;

// ---------------------------------------------------------------------------
// OffloadManager
// ---------------------------------------------------------------------------

/// Manages offload timers for adapters.
///
/// Each adapter can have at most one active timer. Starting a new timer
/// for the same adapter cancels the previous one.
#[derive(Clone)]
pub struct OffloadManager {
    /// The offload timeout duration.
    timeout: Duration,
    /// Active timers, keyed by adapter_id.
    timers: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl OffloadManager {
    /// Create a new offload manager with the given timeout.
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            timers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start an offload timer for an adapter.
    ///
    /// If a timer is already running for this adapter, it is cancelled
    /// and replaced with a new one.
    ///
    /// When the timer expires, `callback` is invoked with the adapter_id.
    pub async fn start_timer(&self, adapter_id: String, callback: OffloadCallback) {
        // Cancel existing timer if any
        self.cancel_timer(&adapter_id).await;

        let timeout = self.timeout;
        let id = adapter_id.clone();
        let timers = self.timers.clone();

        info!(adapter_id = %id, timeout = ?timeout, "starting offload timer");

        let handle = tokio::spawn(async move {
            tokio::time::sleep(timeout).await;

            // Remove ourselves from the timers map before invoking callback
            {
                let mut map = timers.lock().await;
                map.remove(&id);
            }

            info!(adapter_id = %id, "offload timer expired, offloading adapter");
            callback(id);
        });

        self.timers.lock().await.insert(adapter_id, handle);
    }

    /// Cancel an offload timer for an adapter.
    ///
    /// Returns `true` if a timer was cancelled, `false` if no timer was active.
    pub async fn cancel_timer(&self, adapter_id: &str) -> bool {
        let mut timers = self.timers.lock().await;
        if let Some(handle) = timers.remove(adapter_id) {
            handle.abort();
            info!(adapter_id = %adapter_id, "offload timer cancelled");
            true
        } else {
            debug!(adapter_id = %adapter_id, "no offload timer to cancel");
            false
        }
    }

    /// Cancel all active timers.
    pub async fn cancel_all(&self) {
        let mut timers = self.timers.lock().await;
        for (id, handle) in timers.drain() {
            handle.abort();
            warn!(adapter_id = %id, "offload timer cancelled (shutdown)");
        }
    }

    /// Check if a timer is active for the given adapter.
    pub async fn has_timer(&self, adapter_id: &str) -> bool {
        let timers = self.timers.lock().await;
        timers.contains_key(adapter_id)
    }

    /// Get the number of active timers.
    pub async fn active_count(&self) -> usize {
        self.timers.lock().await.len()
    }

    /// Get the configured timeout duration.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl std::fmt::Debug for OffloadManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OffloadManager")
            .field("timeout", &self.timeout)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    #[tokio::test]
    async fn timer_fires_after_timeout() {
        let fired = Arc::new(AtomicBool::new(false));
        let fired_clone = fired.clone();

        // Use short timeouts for real-time tests
        let mgr = OffloadManager::new(Duration::from_millis(50));

        let callback: OffloadCallback = Arc::new(move |_id| {
            fired_clone.store(true, Ordering::SeqCst);
        });

        mgr.start_timer("adapter-1".to_string(), callback).await;

        // Not fired yet
        assert!(!fired.load(Ordering::SeqCst));
        assert!(mgr.has_timer("adapter-1").await);

        // Wait past the timeout
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(fired.load(Ordering::SeqCst));
        assert!(!mgr.has_timer("adapter-1").await);
    }

    #[tokio::test]
    async fn timer_does_not_fire_before_timeout() {
        let fired = Arc::new(AtomicBool::new(false));
        let fired_clone = fired.clone();

        let mgr = OffloadManager::new(Duration::from_millis(200));

        let callback: OffloadCallback = Arc::new(move |_id| {
            fired_clone.store(true, Ordering::SeqCst);
        });

        mgr.start_timer("adapter-1".to_string(), callback).await;

        // Check immediately — should not have fired
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert!(!fired.load(Ordering::SeqCst));
        assert!(mgr.has_timer("adapter-1").await);

        // Clean up
        mgr.cancel_all().await;
    }

    #[tokio::test]
    async fn cancel_prevents_firing() {
        let fired = Arc::new(AtomicBool::new(false));
        let fired_clone = fired.clone();

        let mgr = OffloadManager::new(Duration::from_millis(50));

        let callback: OffloadCallback = Arc::new(move |_id| {
            fired_clone.store(true, Ordering::SeqCst);
        });

        mgr.start_timer("adapter-1".to_string(), callback).await;

        // Cancel before expiry
        let cancelled = mgr.cancel_timer("adapter-1").await;
        assert!(cancelled);
        assert!(!mgr.has_timer("adapter-1").await);

        // Wait past timeout
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(!fired.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn cancel_nonexistent_returns_false() {
        let mgr = OffloadManager::new(Duration::from_millis(50));
        let result = mgr.cancel_timer("nonexistent").await;
        assert!(!result);
    }

    #[tokio::test]
    async fn replace_timer_cancels_old() {
        let fired_count = Arc::new(AtomicUsize::new(0));
        let count_clone = fired_count.clone();

        let mgr = OffloadManager::new(Duration::from_millis(80));

        // Start first timer
        let cb1: OffloadCallback = Arc::new(move |_id| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });
        mgr.start_timer("adapter-1".to_string(), cb1).await;

        // Wait partway (less than timeout)
        tokio::time::sleep(Duration::from_millis(30)).await;

        // Replace with new timer (resets the clock)
        let count_clone2 = fired_count.clone();
        let cb2: OffloadCallback = Arc::new(move |_id| {
            count_clone2.fetch_add(1, Ordering::SeqCst);
        });
        mgr.start_timer("adapter-1".to_string(), cb2).await;

        // Wait — old timer would have fired by now, but new one hasn't
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(fired_count.load(Ordering::SeqCst), 0);

        // Wait for new timer to expire
        tokio::time::sleep(Duration::from_millis(40)).await;

        // Only the new timer should have fired once
        assert_eq!(fired_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn cancel_all_stops_all_timers() {
        let fired = Arc::new(AtomicBool::new(false));

        let mgr = OffloadManager::new(Duration::from_millis(50));

        for i in 0..3 {
            let f = fired.clone();
            let cb: OffloadCallback = Arc::new(move |_id| {
                f.store(true, Ordering::SeqCst);
            });
            mgr.start_timer(format!("adapter-{}", i), cb).await;
        }

        assert_eq!(mgr.active_count().await, 3);

        mgr.cancel_all().await;

        assert_eq!(mgr.active_count().await, 0);

        // Wait past timeout
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(!fired.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn multiple_adapters_independent_timers() {
        let fired_ids = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));

        let mgr = OffloadManager::new(Duration::from_millis(50));

        // Use std::sync::Mutex instead of tokio::Mutex for the callback
        let ids1 = fired_ids.clone();
        let cb1: OffloadCallback = Arc::new(move |id| {
            ids1.lock().unwrap().push(id);
        });
        mgr.start_timer("adapter-1".to_string(), cb1).await;

        let ids2 = fired_ids.clone();
        let cb2: OffloadCallback = Arc::new(move |id| {
            ids2.lock().unwrap().push(id);
        });
        mgr.start_timer("adapter-2".to_string(), cb2).await;

        // Cancel one
        mgr.cancel_timer("adapter-1").await;

        // Wait past timeout
        tokio::time::sleep(Duration::from_millis(100)).await;

        let fired = fired_ids.lock().unwrap();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0], "adapter-2");
    }

    #[test]
    fn offload_manager_timeout() {
        let mgr = OffloadManager::new(Duration::from_secs(300));
        assert_eq!(mgr.timeout(), Duration::from_secs(300));
    }

    #[test]
    fn offload_manager_debug() {
        let mgr = OffloadManager::new(Duration::from_secs(60));
        let debug = format!("{:?}", mgr);
        assert!(debug.contains("OffloadManager"));
        assert!(debug.contains("timeout"));
    }
}
