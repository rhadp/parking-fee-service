use super::*;

/// Verify the ContainerStatus enum has all expected variants.
#[test]
fn test_container_status_variants() {
    let running = ContainerStatus::Running;
    let stopped = ContainerStatus::Stopped;
    let not_found = ContainerStatus::NotFound;
    let unknown = ContainerStatus::Unknown("paused".to_string());

    assert_eq!(running, ContainerStatus::Running);
    assert_eq!(stopped, ContainerStatus::Stopped);
    assert_eq!(not_found, ContainerStatus::NotFound);
    assert_eq!(unknown, ContainerStatus::Unknown("paused".to_string()));
}

/// Verify that ContainerError variants have correct error messages.
#[test]
fn test_container_error_display() {
    let run_err = ContainerError::RunFailed("exit code 125".into());
    assert!(run_err.to_string().contains("exit code 125"));

    let stop_err = ContainerError::StopFailed("timeout".into());
    assert!(stop_err.to_string().contains("timeout"));

    let remove_err = ContainerError::RemoveFailed("permission denied".into());
    assert!(remove_err.to_string().contains("permission denied"));

    let status_err = ContainerError::StatusFailed("no such container".into());
    assert!(status_err.to_string().contains("no such container"));
}

/// Verify PodmanRuntime can be constructed.
#[test]
fn test_podman_runtime_construction() {
    let _runtime = PodmanRuntime::new();
    let _runtime_default = PodmanRuntime::default();
}

/// Verify MockContainerRuntime works for mocking in tests.
#[tokio::test]
async fn test_mock_container_runtime_run() {
    let mut mock = MockContainerRuntime::new();
    mock.expect_run().returning(|_, _| Ok(()));

    let result = mock.run("test-container", "test-image:v1").await;
    assert!(result.is_ok());
}

/// Verify MockContainerRuntime stop works.
#[tokio::test]
async fn test_mock_container_runtime_stop() {
    let mut mock = MockContainerRuntime::new();
    mock.expect_stop().returning(|_| Ok(()));

    let result = mock.stop("test-container").await;
    assert!(result.is_ok());
}

/// Verify MockContainerRuntime remove works.
#[tokio::test]
async fn test_mock_container_runtime_remove() {
    let mut mock = MockContainerRuntime::new();
    mock.expect_remove().returning(|_| Ok(()));

    let result = mock.remove("test-container").await;
    assert!(result.is_ok());
}

/// Verify MockContainerRuntime status works.
#[tokio::test]
async fn test_mock_container_runtime_status() {
    let mut mock = MockContainerRuntime::new();
    mock.expect_status()
        .returning(|_| Ok(ContainerStatus::Running));

    let result = mock.status("test-container").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ContainerStatus::Running);
}

/// Verify MockContainerRuntime can simulate run failure.
#[tokio::test]
async fn test_mock_container_runtime_run_failure() {
    let mut mock = MockContainerRuntime::new();
    mock.expect_run()
        .returning(|_, _| Err(ContainerError::RunFailed("exit code 125".into())));

    let result = mock.run("test-container", "test-image:v1").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ContainerError::RunFailed(msg) => {
            assert!(msg.contains("exit code 125"));
        }
        other => panic!("expected RunFailed, got: {:?}", other),
    }
}

/// Verify MockContainerRuntime can return NotFound status.
#[tokio::test]
async fn test_mock_container_runtime_status_not_found() {
    let mut mock = MockContainerRuntime::new();
    mock.expect_status()
        .returning(|_| Ok(ContainerStatus::NotFound));

    let result = mock.status("nonexistent").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ContainerStatus::NotFound);
}
