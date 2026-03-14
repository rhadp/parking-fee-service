use std::sync::Arc;

use crate::container::ContainerRuntime;
use crate::model::AdapterState;
use crate::state::StateManager;

/// Errors returned by service-level operations, mapped to gRPC status codes
/// by the gRPC handler.
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceError {
    /// gRPC INVALID_ARGUMENT
    InvalidArgument(String),
    /// gRPC UNAVAILABLE
    Unavailable(String),
    /// gRPC FAILED_PRECONDITION
    FailedPrecondition(String),
    /// gRPC INTERNAL
    Internal(String),
    /// gRPC NOT_FOUND
    NotFound(String),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::InvalidArgument(m) => write!(f, "INVALID_ARGUMENT: {}", m),
            ServiceError::Unavailable(m) => write!(f, "UNAVAILABLE: {}", m),
            ServiceError::FailedPrecondition(m) => write!(f, "FAILED_PRECONDITION: {}", m),
            ServiceError::Internal(m) => write!(f, "INTERNAL: {}", m),
            ServiceError::NotFound(m) => write!(f, "NOT_FOUND: {}", m),
        }
    }
}

impl std::error::Error for ServiceError {}

/// Value returned by `install_adapter` on success.
#[derive(Debug)]
pub struct InstallResponse {
    pub job_id: String,
    pub adapter_id: String,
    /// The state at the moment the adapter record was created (always
    /// `AdapterState::Downloading`).
    pub initial_state: AdapterState,
}

/// Install an adapter:
/// 1. Validate inputs.
/// 2. Enforce single-adapter constraint (stop any currently RUNNING adapter).
/// 3. Create adapter record in DOWNLOADING state.
/// 4. Pull image → verify checksum → transition to INSTALLING → run container
///    → transition to RUNNING.
///
/// Returns `(job_id, adapter_id, DOWNLOADING)` reflecting the initial state.
/// The caller (gRPC handler) may choose to spawn this as a background task
/// and return the initial response immediately; for unit testing the full
/// pipeline is awaited synchronously.
pub async fn install_adapter(
    _manager: Arc<StateManager>,
    _runtime: Arc<dyn ContainerRuntime>,
    _image_ref: &str,
    _checksum_sha256: &str,
) -> Result<InstallResponse, ServiceError> {
    todo!("implement install_adapter")
}

/// Remove an adapter:
/// 1. Verify adapter exists.
/// 2. Stop container if RUNNING → transition to STOPPED.
/// 3. Transition to OFFLOADING.
/// 4. Remove container, remove image.
/// 5. Delete from state.
pub async fn remove_adapter(
    _manager: Arc<StateManager>,
    _runtime: Arc<dyn ContainerRuntime>,
    _adapter_id: &str,
) -> Result<(), ServiceError> {
    todo!("implement remove_adapter")
}

// ---------------------------------------------------------------------------
// Edge-case tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::MockContainerRuntime;
    use std::sync::Arc;

    fn manager() -> Arc<StateManager> {
        Arc::new(StateManager::new())
    }

    fn runtime_ok(checksum: &str) -> Arc<dyn ContainerRuntime> {
        Arc::new(MockContainerRuntime::new().with_digest(checksum))
    }

    async fn install_ok(
        m: &Arc<StateManager>,
        r: Arc<dyn ContainerRuntime>,
        image_ref: &str,
        checksum: &str,
    ) -> InstallResponse {
        install_adapter(Arc::clone(m), r, image_ref, checksum)
            .await
            .expect("install_ok: should succeed")
    }

    // TS-07-E1: Empty image_ref returns INVALID_ARGUMENT
    #[tokio::test]
    async fn test_empty_image_ref() {
        let m = manager();
        let r = runtime_ok("sha256:abc");
        let err = install_adapter(Arc::clone(&m), r, "", "sha256:abc")
            .await
            .expect_err("empty image_ref must fail");
        assert!(
            matches!(err, ServiceError::InvalidArgument(_)),
            "expected INVALID_ARGUMENT, got {:?}",
            err
        );
    }

    // TS-07-E1: Empty checksum returns INVALID_ARGUMENT
    #[tokio::test]
    async fn test_empty_checksum() {
        let m = manager();
        let r = runtime_ok("sha256:abc");
        let err = install_adapter(Arc::clone(&m), r, "image:v1", "")
            .await
            .expect_err("empty checksum must fail");
        assert!(
            matches!(err, ServiceError::InvalidArgument(_)),
            "expected INVALID_ARGUMENT, got {:?}",
            err
        );
    }

    // TS-07-E2: Image pull failure → ERROR state + UNAVAILABLE
    #[tokio::test]
    async fn test_pull_failure() {
        let m = manager();
        let rt = Arc::new(MockContainerRuntime::new().with_digest("sha256:abc"));
        rt.set_pull_error(true);

        let err = install_adapter(
            Arc::clone(&m),
            Arc::clone(&rt) as Arc<dyn ContainerRuntime>,
            "registry.io/repo/adapter:v1",
            "sha256:abc",
        )
        .await
        .expect_err("pull failure must return error");

        assert!(
            matches!(err, ServiceError::Unavailable(_)),
            "expected UNAVAILABLE, got {:?}",
            err
        );

        let info = m.get("adapter-v1").expect("adapter must still exist");
        assert_eq!(
            info.state,
            AdapterState::Error,
            "adapter must be in ERROR state"
        );
    }

    // TS-07-E3: Checksum mismatch → ERROR state + image removed + FAILED_PRECONDITION
    #[tokio::test]
    async fn test_checksum_mismatch() {
        let m = manager();
        let rt = Arc::new(MockContainerRuntime::new().with_digest("sha256:wrong"));

        let err = install_adapter(
            Arc::clone(&m),
            Arc::clone(&rt) as Arc<dyn ContainerRuntime>,
            "registry.io/repo/adapter:v1",
            "sha256:expected",
        )
        .await
        .expect_err("checksum mismatch must return error");

        assert!(
            matches!(err, ServiceError::FailedPrecondition(_)),
            "expected FAILED_PRECONDITION, got {:?}",
            err
        );
        assert!(
            rt.was_remove_image_called(),
            "remove_image must be called on checksum mismatch"
        );
        let info = m.get("adapter-v1").expect("adapter must still exist");
        assert_eq!(info.state, AdapterState::Error);
    }

    // TS-07-E4: Container start failure → ERROR state + INTERNAL
    #[tokio::test]
    async fn test_container_start_failure() {
        let m = manager();
        let rt = Arc::new(MockContainerRuntime::new().with_digest("sha256:abc"));
        rt.set_run_error(true);

        let err = install_adapter(
            Arc::clone(&m),
            Arc::clone(&rt) as Arc<dyn ContainerRuntime>,
            "registry.io/repo/adapter:v1",
            "sha256:abc",
        )
        .await
        .expect_err("run failure must return error");

        assert!(
            matches!(err, ServiceError::Internal(_)),
            "expected INTERNAL, got {:?}",
            err
        );
        let info = m.get("adapter-v1").expect("adapter must still exist");
        assert_eq!(info.state, AdapterState::Error);
    }

    // TS-07-E5: Stop running adapter fails → INTERNAL, new adapter not created
    #[tokio::test]
    async fn test_stop_running_fails() {
        let m = manager();
        let rt1 = Arc::new(MockContainerRuntime::new().with_digest("sha256:cs1"));

        // Install first adapter to RUNNING
        install_ok(
            &m,
            Arc::clone(&rt1) as Arc<dyn ContainerRuntime>,
            "registry.io/repo/old-adapter:v1",
            "sha256:cs1",
        )
        .await;

        // Configure stop to fail for second install attempt
        let rt2 = Arc::new(MockContainerRuntime::new().with_digest("sha256:cs2"));
        rt2.set_stop_error(true);

        let err = install_adapter(
            Arc::clone(&m),
            Arc::clone(&rt2) as Arc<dyn ContainerRuntime>,
            "registry.io/repo/new-adapter:v1",
            "sha256:cs2",
        )
        .await
        .expect_err("stop failure must abort new install");

        assert!(
            matches!(err, ServiceError::Internal(_)),
            "expected INTERNAL, got {:?}",
            err
        );
        assert!(
            m.get("new-adapter-v1").is_none(),
            "new adapter must NOT be created when stop fails"
        );
    }

    // TS-07-E7: Remove unknown adapter → NOT_FOUND
    #[tokio::test]
    async fn test_remove_unknown_adapter() {
        let m = manager();
        let rt = runtime_ok("sha256:abc");

        let err = remove_adapter(Arc::clone(&m), rt, "nonexistent-adapter")
            .await
            .expect_err("removing unknown adapter must fail");

        assert!(
            matches!(err, ServiceError::NotFound(_)),
            "expected NOT_FOUND, got {:?}",
            err
        );
    }

    // TS-07-E8: Container removal failure → ERROR state + INTERNAL
    #[tokio::test]
    async fn test_container_removal_failure() {
        let m = manager();
        let rt = Arc::new(MockContainerRuntime::new().with_digest("sha256:abc"));

        // Install and transition to STOPPED manually
        install_ok(
            &m,
            Arc::clone(&rt) as Arc<dyn ContainerRuntime>,
            "registry.io/repo/adapter:v1",
            "sha256:abc",
        )
        .await;

        // Now configure remove to fail
        rt.set_remove_error(true);

        let err = remove_adapter(
            Arc::clone(&m),
            Arc::clone(&rt) as Arc<dyn ContainerRuntime>,
            "adapter-v1",
        )
        .await
        .expect_err("removal failure must return error");

        assert!(
            matches!(err, ServiceError::Internal(_)),
            "expected INTERNAL, got {:?}",
            err
        );
        let info = m.get("adapter-v1").expect("adapter must still exist");
        assert_eq!(info.state, AdapterState::Error);
    }
}

// ---------------------------------------------------------------------------
// Property tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::container::MockContainerRuntime;
    use std::sync::Arc;

    proptest::proptest! {
        // TS-07-P2: Single Adapter Constraint
        // At most one adapter is in RUNNING state after any sequence of installs.
        #[test]
        #[ignore]
        fn proptest_single_adapter_constraint(
            n in 1usize..=5usize,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = Arc::new(StateManager::new());

                for i in 0..n {
                    let image_ref = format!("registry.io/repo/adapter-{}:v1", i);
                    let checksum = format!("sha256:cs{}", i);
                    let runtime = Arc::new(
                        MockContainerRuntime::new().with_digest(&checksum),
                    );
                    let _ = install_adapter(
                        Arc::clone(&manager),
                        runtime as Arc<dyn ContainerRuntime>,
                        &image_ref,
                        &checksum,
                    )
                    .await;
                }

                let running: Vec<_> = manager
                    .list()
                    .into_iter()
                    .filter(|a| a.state == crate::model::AdapterState::Running)
                    .collect();
                proptest::prop_assert!(
                    running.len() <= 1,
                    "at most one adapter must be RUNNING; found {}",
                    running.len()
                );
                Ok(())
            })?;
        }

        // TS-07-P3: Checksum Gate
        // Mismatched checksums always result in ERROR, never RUNNING.
        #[test]
        #[ignore]
        fn proptest_checksum_gate(
            actual   in "[a-f0-9]{8}",
            provided in "[a-f0-9]{8}",
        ) {
            proptest::prop_assume!(actual != provided);

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = Arc::new(StateManager::new());
                let runtime = Arc::new(
                    MockContainerRuntime::new().with_digest(&format!("sha256:{}", actual)),
                );

                let _ = install_adapter(
                    Arc::clone(&manager),
                    Arc::clone(&runtime) as Arc<dyn ContainerRuntime>,
                    "registry.io/repo/adapter:v1",
                    &format!("sha256:{}", provided),
                )
                .await;

                let info = manager.get("adapter-v1");
                if let Some(info) = info {
                    proptest::prop_assert_eq!(
                        info.state,
                        crate::model::AdapterState::Error,
                        "mismatched checksum must result in ERROR state"
                    );
                }
                proptest::prop_assert!(
                    !runtime.was_run_called(),
                    "container must NOT be started when checksum mismatches"
                );
                Ok(())
            })?;
        }
    }
}
