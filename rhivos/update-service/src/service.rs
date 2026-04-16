use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

use crate::adapter::{AdapterEntry, AdapterState, AdapterStateEvent};
use crate::podman::PodmanExecutor;
use crate::state::{StateError, StateManager};

#[derive(Debug)]
pub struct ServiceError {
    pub code: ServiceErrorCode,
    pub message: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ServiceErrorCode {
    InvalidArgument,
    NotFound,
    Internal,
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for ServiceError {}

#[derive(Debug)]
pub struct InstallAdapterResponse {
    pub job_id: String,
    pub adapter_id: String,
    pub state: AdapterState,
}

pub struct UpdateService<P: PodmanExecutor + Send + Sync + 'static> {
    pub state: Arc<StateManager>,
    pub podman: Arc<P>,
    pub broadcaster: broadcast::Sender<AdapterStateEvent>,
    pub inactivity_timeout: Duration,
}

impl<P: PodmanExecutor + Send + Sync + 'static> UpdateService<P> {
    pub fn new(
        state: Arc<StateManager>,
        podman: Arc<P>,
        broadcaster: broadcast::Sender<AdapterStateEvent>,
        inactivity_timeout: Duration,
    ) -> Self {
        Self {
            state,
            podman,
            broadcaster,
            inactivity_timeout,
        }
    }

    /// InstallAdapter: validate inputs, derive ID, stop running adapter if any,
    /// create entry, return immediately with DOWNLOADING state, spawn async install task.
    pub async fn install_adapter(
        &self,
        _image_ref: &str,
        _checksum_sha256: &str,
    ) -> Result<InstallAdapterResponse, ServiceError> {
        todo!("implement install_adapter")
    }

    /// RemoveAdapter: stop (if running) + rm + rmi, remove from state.
    pub async fn remove_adapter(&self, _adapter_id: &str) -> Result<(), ServiceError> {
        todo!("implement remove_adapter")
    }

    /// GetAdapterStatus: return current adapter entry.
    pub fn get_adapter_status(&self, _adapter_id: &str) -> Result<AdapterEntry, ServiceError> {
        todo!("implement get_adapter_status")
    }

    /// ListAdapters: return all adapters.
    pub fn list_adapters(&self) -> Vec<AdapterEntry> {
        todo!("implement list_adapters")
    }

    /// Subscribe to state events.
    pub fn watch_adapter_states(&self) -> broadcast::Receiver<AdapterStateEvent> {
        self.broadcaster.subscribe()
    }
}

// Suppress unused import warning for StateError which is used in later task groups
#[allow(dead_code)]
fn _use_state_error(_: StateError) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::AdapterState;
    use crate::podman::{MockPodmanExecutor, PodmanError};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::broadcast;

    const IMAGE_REF_A: &str = "us-docker.pkg.dev/sdv-demo/adapters/parkhaus-munich:v1.0.0";
    const CHECKSUM_A: &str = "sha256:abc123";
    const ADAPTER_ID_A: &str = "parkhaus-munich-v1.0.0";

    const IMAGE_REF_B: &str = "us-docker.pkg.dev/sdv-demo/adapters/another-adapter:v2.0.0";
    const CHECKSUM_B: &str = "sha256:def456";
    const ADAPTER_ID_B: &str = "another-adapter-v2.0.0";

    fn make_service(
        mock: Arc<MockPodmanExecutor>,
    ) -> (Arc<StateManager>, UpdateService<MockPodmanExecutor>) {
        let (tx, _rx) = broadcast::channel(100);
        let state = Arc::new(StateManager::new(tx.clone()));
        let svc = UpdateService::new(state.clone(), mock, tx, Duration::from_secs(86400));
        (state, svc)
    }

    // TS-07-7: Single adapter constraint stops running adapter
    #[tokio::test]
    async fn test_single_adapter_stops_running() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM_A.to_string()));
        mock.set_run_result(Ok(()));
        let mock_clone = mock.clone();
        let (state, svc) = make_service(mock);

        // Install adapter A
        svc.install_adapter(IMAGE_REF_A, CHECKSUM_A).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(
            state.get_adapter(ADAPTER_ID_A).unwrap().state,
            AdapterState::Running
        );

        // Now update inspect result for adapter B
        mock_clone.set_inspect_result(Ok(CHECKSUM_B.to_string()));

        // Install adapter B - should stop adapter A first
        svc.install_adapter(IMAGE_REF_B, CHECKSUM_B).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        assert!(mock_clone.stop_calls().contains(&ADAPTER_ID_A.to_string()));
        assert_eq!(
            state.get_adapter(ADAPTER_ID_A).unwrap().state,
            AdapterState::Stopped
        );
        assert_eq!(
            state.get_adapter(ADAPTER_ID_B).unwrap().state,
            AdapterState::Running
        );
    }

    // TS-07-E6: Stop running adapter fails but install proceeds
    #[tokio::test]
    async fn test_stop_failure_install_proceeds() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM_A.to_string()));
        mock.set_run_result(Ok(()));
        let mock_clone = mock.clone();
        let (state, svc) = make_service(mock);

        // Install adapter A (gets to RUNNING)
        svc.install_adapter(IMAGE_REF_A, CHECKSUM_A).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(
            state.get_adapter(ADAPTER_ID_A).unwrap().state,
            AdapterState::Running
        );

        // Configure stop to fail for adapter A
        mock_clone.set_stop_result_for(ADAPTER_ID_A, Err(PodmanError::new("timeout")));
        mock_clone.set_inspect_result(Ok(CHECKSUM_B.to_string()));

        // Install adapter B - stop of A fails but B should still install
        svc.install_adapter(IMAGE_REF_B, CHECKSUM_B).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        assert_eq!(
            state.get_adapter(ADAPTER_ID_A).unwrap().state,
            AdapterState::Error
        );
        assert_eq!(
            state.get_adapter(ADAPTER_ID_B).unwrap().state,
            AdapterState::Running
        );
    }

    // TS-07-E8: GetAdapterStatus unknown ID returns NOT_FOUND error
    #[tokio::test]
    async fn test_get_status_unknown_adapter() {
        let mock = Arc::new(MockPodmanExecutor::new());
        let (_state, svc) = make_service(mock);

        let result = svc.get_adapter_status("nonexistent-adapter");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, ServiceErrorCode::NotFound);
        assert!(err.message.contains("adapter not found"));
    }

    // TS-07-E9: ListAdapters returns empty when none installed
    #[tokio::test]
    async fn test_list_adapters_empty() {
        let mock = Arc::new(MockPodmanExecutor::new());
        let (_state, svc) = make_service(mock);

        let list = svc.list_adapters();
        assert!(list.is_empty());
    }

    // TS-07-E10: RemoveAdapter unknown ID returns NOT_FOUND
    #[tokio::test]
    async fn test_remove_unknown_adapter() {
        let mock = Arc::new(MockPodmanExecutor::new());
        let (_state, svc) = make_service(mock);

        let result = svc.remove_adapter("nonexistent-adapter").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, ServiceErrorCode::NotFound);
        assert!(err.message.contains("adapter not found"));
    }

    // TS-07-E11: Podman removal failure returns INTERNAL
    #[tokio::test]
    async fn test_removal_failure_internal() {
        let mock = Arc::new(MockPodmanExecutor::new());
        mock.set_pull_result(Ok(()));
        mock.set_inspect_result(Ok(CHECKSUM_A.to_string()));
        mock.set_run_result(Ok(()));
        let mock_clone = mock.clone();
        let (state, svc) = make_service(mock);

        // Install adapter A to get it to RUNNING
        svc.install_adapter(IMAGE_REF_A, CHECKSUM_A).await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Set rm to fail
        mock_clone.set_rm_result(Err(PodmanError::new("container in use")));

        let result = svc.remove_adapter(ADAPTER_ID_A).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, ServiceErrorCode::Internal);

        let adapter = state.get_adapter(ADAPTER_ID_A).unwrap();
        assert_eq!(adapter.state, AdapterState::Error);
    }
}

// TS-07-P2: Single adapter invariant — at most one RUNNING at any time
#[cfg(test)]
mod proptest_tests {
    use super::*;
    use crate::podman::MockPodmanExecutor;
    use proptest::prelude::*;

    fn make_service_for_proptest(
        mock: Arc<MockPodmanExecutor>,
    ) -> (Arc<StateManager>, UpdateService<MockPodmanExecutor>) {
        let (tx, _rx) = tokio::sync::broadcast::channel(1000);
        let state = Arc::new(StateManager::new(tx.clone()));
        let svc = UpdateService::new(
            state.clone(),
            mock,
            tx,
            std::time::Duration::from_secs(86400),
        );
        (state, svc)
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(10))]

        // TS-07-P2: Single adapter invariant
        #[test]
        #[ignore = "proptest: run with --include-ignored"]
        fn proptest_single_adapter_invariant(
            adapters in proptest::collection::vec("[a-z][a-z0-9]{2,8}", 1..4usize),
        ) {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            // Collect running counts from async context, assert outside
            let running_counts: Vec<usize> = rt.block_on(async {
                let mock = Arc::new(MockPodmanExecutor::new());
                mock.set_pull_result(Ok(()));
                mock.set_inspect_result(Ok("sha256:test".to_string()));
                mock.set_run_result(Ok(()));
                mock.set_stop_result(Ok(()));
                mock.set_wait_result(Ok(0));
                let (state, svc) = make_service_for_proptest(mock);
                let mut counts = Vec::new();
                for name in &adapters {
                    let image_ref = format!("registry.example.com/{name}:v1");
                    let _ = svc.install_adapter(&image_ref, "sha256:test").await;
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    counts.push(
                        state
                            .list_adapters()
                            .into_iter()
                            .filter(|a| a.state == AdapterState::Running)
                            .count(),
                    );
                }
                counts
            });
            for count in running_counts {
                prop_assert!(count <= 1, "More than 1 RUNNING adapter: {count}");
            }
        }

        // TS-07-P4: Event delivery completeness — all subscribers receive same events
        #[test]
        #[ignore = "proptest: run with --include-ignored"]
        fn proptest_event_delivery_completeness(n_subscribers in 1usize..4) {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            // Collect event counts per subscriber from async context, assert outside
            let event_counts: Vec<usize> = rt.block_on(async {
                let mock = Arc::new(MockPodmanExecutor::new());
                mock.set_pull_result(Ok(()));
                mock.set_inspect_result(Ok("sha256:abc".to_string()));
                mock.set_run_result(Ok(()));
                mock.set_wait_result(Ok(0));
                let (tx, _rx) = tokio::sync::broadcast::channel(1000);
                let state = Arc::new(StateManager::new(tx.clone()));
                let svc = UpdateService::new(
                    state.clone(),
                    mock,
                    tx,
                    std::time::Duration::from_secs(86400),
                );
                let mut rxs: Vec<tokio::sync::broadcast::Receiver<_>> = (0..n_subscribers)
                    .map(|_| svc.watch_adapter_states())
                    .collect();
                let _ = svc.install_adapter("registry.example.com/adapter-a:v1", "sha256:abc").await;
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                rxs.iter_mut()
                    .map(|rx| {
                        let mut count = 0;
                        while rx.try_recv().is_ok() { count += 1; }
                        count
                    })
                    .collect()
            });
            if event_counts.len() > 1 {
                let first = event_counts[0];
                for count in &event_counts[1..] {
                    prop_assert_eq!(*count, first, "Subscriber received different event count");
                }
            }
        }
    }
}
