use super::*;

/// PodmanRuntime can be constructed with new() and default().
#[test]
fn test_podman_runtime_construction() {
    let _rt = PodmanRuntime::new();
    let _rt_default = PodmanRuntime::default();
}

/// parse_container_status correctly maps "running" to Running.
#[test]
fn test_parse_status_running() {
    assert_eq!(parse_container_status("running"), ContainerStatus::Running);
    assert_eq!(parse_container_status("Running"), ContainerStatus::Running);
    assert_eq!(
        parse_container_status("  running\n"),
        ContainerStatus::Running
    );
}

/// parse_container_status correctly maps stopped-like states.
#[test]
fn test_parse_status_stopped() {
    assert_eq!(parse_container_status("exited"), ContainerStatus::Stopped);
    assert_eq!(parse_container_status("stopped"), ContainerStatus::Stopped);
    assert_eq!(parse_container_status("created"), ContainerStatus::Stopped);
    assert_eq!(parse_container_status("paused"), ContainerStatus::Stopped);
    assert_eq!(parse_container_status("dead"), ContainerStatus::Stopped);
}

/// parse_container_status returns NotFound for empty strings.
#[test]
fn test_parse_status_not_found() {
    assert_eq!(parse_container_status(""), ContainerStatus::NotFound);
    assert_eq!(parse_container_status("  "), ContainerStatus::NotFound);
}

/// parse_container_status returns Unknown for unrecognized values.
#[test]
fn test_parse_status_unknown() {
    assert_eq!(
        parse_container_status("restarting"),
        ContainerStatus::Unknown("restarting".to_string())
    );
    assert_eq!(
        parse_container_status("removing"),
        ContainerStatus::Unknown("removing".to_string())
    );
}

/// ContainerError Display implementations produce expected messages.
#[test]
fn test_container_error_display() {
    let err = ContainerError::StartFailed("exit code 125".to_string());
    assert_eq!(
        err.to_string(),
        "container failed to start: exit code 125"
    );

    let err = ContainerError::StopFailed("timeout".to_string());
    assert_eq!(err.to_string(), "container failed to stop: timeout");

    let err = ContainerError::RemoveFailed("permission denied".to_string());
    assert_eq!(
        err.to_string(),
        "container removal failed: permission denied"
    );

    let err = ContainerError::StatusFailed("inspect error".to_string());
    assert_eq!(
        err.to_string(),
        "container status failed: inspect error"
    );

    let err = ContainerError::NotFound("my-container".to_string());
    assert_eq!(err.to_string(), "container not found: my-container");
}

/// ContainerStatus enum variants are distinct and comparable.
#[test]
fn test_container_status_equality() {
    assert_eq!(ContainerStatus::Running, ContainerStatus::Running);
    assert_eq!(ContainerStatus::Stopped, ContainerStatus::Stopped);
    assert_eq!(ContainerStatus::NotFound, ContainerStatus::NotFound);
    assert_ne!(ContainerStatus::Running, ContainerStatus::Stopped);
    assert_eq!(
        ContainerStatus::Unknown("foo".into()),
        ContainerStatus::Unknown("foo".into())
    );
    assert_ne!(
        ContainerStatus::Unknown("foo".into()),
        ContainerStatus::Unknown("bar".into())
    );
}

/// MockContainerRuntime can be constructed and expectations set.
/// This validates the #[automock] derivation works correctly.
#[tokio::test]
async fn test_mock_container_runtime() {
    let mut mock = MockContainerRuntime::new();
    mock.expect_run().returning(|_, _| Ok(()));
    mock.expect_stop().returning(|_| Ok(()));
    mock.expect_remove().returning(|_| Ok(()));
    mock.expect_status()
        .returning(|_| Ok(ContainerStatus::Running));

    assert!(mock.run("test", "image:v1").await.is_ok());
    assert!(mock.stop("test").await.is_ok());
    assert!(mock.remove("test").await.is_ok());
    assert_eq!(
        mock.status("test").await.unwrap(),
        ContainerStatus::Running
    );
}

/// MockContainerRuntime can simulate start failure for TS-07-E3.
#[tokio::test]
async fn test_mock_container_start_failure() {
    let mut mock = MockContainerRuntime::new();
    mock.expect_run()
        .returning(|_, _| Err(ContainerError::StartFailed("exit code 125".into())));

    let result = mock.run("test", "image:v1").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ContainerError::StartFailed(msg) => {
            assert!(msg.contains("125"), "error should contain exit code");
        }
        other => panic!("expected StartFailed, got: {:?}", other),
    }
}
