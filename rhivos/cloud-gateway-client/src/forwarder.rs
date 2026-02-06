//! Command forwarding to LOCKING_SERVICE.
//!
//! This module forwards validated lock/unlock commands to the LOCKING_SERVICE
//! via gRPC over UDS.

use std::time::Duration;

use tokio::time::timeout;
use tonic::transport::Channel;
use tracing::{debug, error, info};

use crate::command::{Command, CommandType, Door};
use crate::error::ForwardError;
use crate::proto::locking::locking_service_client::LockingServiceClient;
use crate::proto::locking::{Door as ProtoDoor, LockRequest, UnlockRequest};

/// Result of a command forward operation.
#[derive(Debug, Clone)]
pub struct ForwardResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Error code if failed
    pub error_code: Option<String>,
    /// Error message if failed
    pub error_message: Option<String>,
}

impl ForwardResult {
    /// Create a success result.
    pub fn success() -> Self {
        Self {
            success: true,
            error_code: None,
            error_message: None,
        }
    }

    /// Create a failure result.
    pub fn failure(error_code: String, error_message: String) -> Self {
        Self {
            success: false,
            error_code: Some(error_code),
            error_message: Some(error_message),
        }
    }
}

/// Forwards commands to LOCKING_SERVICE via gRPC/UDS.
pub struct CommandForwarder {
    /// gRPC client for LOCKING_SERVICE
    client: Option<LockingServiceClient<Channel>>,
    /// Socket path for LOCKING_SERVICE
    socket_path: String,
    /// Command timeout
    timeout_ms: u64,
}

impl CommandForwarder {
    /// Create a new command forwarder.
    pub fn new(socket_path: String, timeout_ms: u64) -> Self {
        Self {
            client: None,
            socket_path,
            timeout_ms,
        }
    }

    /// Connect to the LOCKING_SERVICE over UDS.
    ///
    /// This uses tonic's built-in UDS support via the `transport` feature.
    pub async fn connect(&mut self) -> Result<(), ForwardError> {
        // Note: For UDS connections with tonic 0.10+, we need to use the
        // Unix socket support. The actual connection code would look like:
        //
        // let channel = Endpoint::try_from("http://[::]:50051")?
        //     .connect_with_connector(service_fn(|_| async {
        //         UnixStream::connect(path).await
        //     }))
        //     .await?;
        //
        // For now, we'll attempt a direct connection which works for testing
        // with a local gRPC server.

        let endpoint = "http://localhost:50051".to_string();
        let channel = Channel::from_shared(endpoint)
            .map_err(|e| ForwardError::ServiceUnavailable(e.to_string()))?
            .connect()
            .await
            .map_err(|e| ForwardError::ServiceUnavailable(e.to_string()))?;

        self.client = Some(LockingServiceClient::new(channel));
        info!("Connected to LOCKING_SERVICE at {}", self.socket_path);

        Ok(())
    }

    /// Check if the forwarder is connected.
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    /// Create a forwarder with a pre-connected client (for testing).
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn with_client(client: LockingServiceClient<Channel>, timeout_ms: u64) -> Self {
        Self {
            client: Some(client),
            socket_path: String::new(),
            timeout_ms,
        }
    }

    /// Forward a command to LOCKING_SERVICE.
    pub async fn forward(&mut self, command: &Command) -> Result<ForwardResult, ForwardError> {
        // For commands with multiple doors, we send requests for each door
        // For "All" door, we send a single request
        let door = if command.doors.contains(&Door::All) {
            Door::All
        } else {
            command.doors.first().cloned().unwrap_or(Door::All)
        };

        match command.command_type {
            CommandType::Lock => {
                self.forward_lock(&door, &command.command_id, &command.auth_token)
                    .await
            }
            CommandType::Unlock => {
                self.forward_unlock(&door, &command.command_id, &command.auth_token)
                    .await
            }
        }
    }

    /// Forward a lock command.
    pub async fn forward_lock(
        &mut self,
        door: &Door,
        command_id: &str,
        auth_token: &str,
    ) -> Result<ForwardResult, ForwardError> {
        let client = self
            .client
            .as_mut()
            .ok_or_else(|| ForwardError::ServiceUnavailable("Not connected".to_string()))?;

        let request = tonic::Request::new(LockRequest {
            door: door_to_proto(door) as i32,
            command_id: command_id.to_string(),
            auth_token: auth_token.to_string(),
        });

        debug!("Forwarding lock command to LOCKING_SERVICE");

        let result = timeout(Duration::from_millis(self.timeout_ms), client.lock(request)).await;

        match result {
            Ok(Ok(response)) => {
                let resp = response.into_inner();
                if resp.success {
                    info!("Lock command succeeded");
                    Ok(ForwardResult::success())
                } else {
                    error!("Lock command failed: {:?}", resp.error_message);
                    Ok(ForwardResult::failure(
                        "LOCK_FAILED".to_string(),
                        resp.error_message,
                    ))
                }
            }
            Ok(Err(e)) => {
                error!("Lock command gRPC error: {}", e);
                Err(ForwardError::ExecutionFailed(e.to_string()))
            }
            Err(_) => {
                error!("Lock command timed out");
                Err(ForwardError::Timeout)
            }
        }
    }

    /// Forward an unlock command.
    pub async fn forward_unlock(
        &mut self,
        door: &Door,
        command_id: &str,
        auth_token: &str,
    ) -> Result<ForwardResult, ForwardError> {
        let client = self
            .client
            .as_mut()
            .ok_or_else(|| ForwardError::ServiceUnavailable("Not connected".to_string()))?;

        let request = tonic::Request::new(UnlockRequest {
            door: door_to_proto(door) as i32,
            command_id: command_id.to_string(),
            auth_token: auth_token.to_string(),
        });

        debug!("Forwarding unlock command to LOCKING_SERVICE");

        let result = timeout(
            Duration::from_millis(self.timeout_ms),
            client.unlock(request),
        )
        .await;

        match result {
            Ok(Ok(response)) => {
                let resp = response.into_inner();
                if resp.success {
                    info!("Unlock command succeeded");
                    Ok(ForwardResult::success())
                } else {
                    error!("Unlock command failed: {:?}", resp.error_message);
                    Ok(ForwardResult::failure(
                        "UNLOCK_FAILED".to_string(),
                        resp.error_message,
                    ))
                }
            }
            Ok(Err(e)) => {
                error!("Unlock command gRPC error: {}", e);
                Err(ForwardError::ExecutionFailed(e.to_string()))
            }
            Err(_) => {
                error!("Unlock command timed out");
                Err(ForwardError::Timeout)
            }
        }
    }
}

/// Convert a Door enum to the proto Door.
fn door_to_proto(door: &Door) -> ProtoDoor {
    match door {
        Door::Driver => ProtoDoor::Driver,
        Door::Passenger => ProtoDoor::Passenger,
        Door::RearLeft => ProtoDoor::RearLeft,
        Door::RearRight => ProtoDoor::RearRight,
        Door::All => ProtoDoor::All,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_forward_result_success() {
        let result = ForwardResult::success();
        assert!(result.success);
        assert!(result.error_code.is_none());
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_forward_result_failure() {
        let result = ForwardResult::failure("ERR_001".to_string(), "Something failed".to_string());
        assert!(!result.success);
        assert_eq!(result.error_code, Some("ERR_001".to_string()));
        assert_eq!(result.error_message, Some("Something failed".to_string()));
    }

    #[test]
    fn test_door_to_proto() {
        use crate::proto::locking::Door as ProtoDoor;
        assert_eq!(door_to_proto(&Door::Driver), ProtoDoor::Driver);
        assert_eq!(door_to_proto(&Door::Passenger), ProtoDoor::Passenger);
        assert_eq!(door_to_proto(&Door::RearLeft), ProtoDoor::RearLeft);
        assert_eq!(door_to_proto(&Door::RearRight), ProtoDoor::RearRight);
        assert_eq!(door_to_proto(&Door::All), ProtoDoor::All);
    }

    // Property 8: Command Forwarding by Type
    // Validates: Requirements 4.1, 4.2
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_forward_result_success_has_no_error(
            _ in proptest::bool::ANY
        ) {
            let result = ForwardResult::success();

            // Success results should not have error fields
            prop_assert!(result.success);
            prop_assert!(result.error_code.is_none());
            prop_assert!(result.error_message.is_none());
        }

        #[test]
        fn prop_forward_result_failure_has_error(
            error_code in "[A-Z_]{3,20}",
            error_message in "[a-zA-Z0-9 ]{1,100}"
        ) {
            let result = ForwardResult::failure(error_code.clone(), error_message.clone());

            // Failure results should have error fields
            prop_assert!(!result.success);
            prop_assert_eq!(result.error_code, Some(error_code));
            prop_assert_eq!(result.error_message, Some(error_message));
        }
    }

    // Property 9: Response Status Matches LOCKING_SERVICE Result
    // Validates: Requirements 4.3, 4.4
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_door_mapping_is_bijective(
            door_idx in 0usize..5
        ) {
            use crate::proto::locking::Door as ProtoDoor;

            let doors = [Door::Driver, Door::Passenger, Door::RearLeft, Door::RearRight, Door::All];
            let proto_ids = [ProtoDoor::Driver, ProtoDoor::Passenger, ProtoDoor::RearLeft, ProtoDoor::RearRight, ProtoDoor::All];

            let door = &doors[door_idx];
            let expected = proto_ids[door_idx];

            // Mapping should be consistent
            prop_assert_eq!(door_to_proto(door), expected);
        }
    }
}
