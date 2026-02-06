//! gRPC service implementation for LOCKING_SERVICE.
//!
//! This module implements the LockingService gRPC interface, wiring together
//! authentication, safety validation, execution, and state publication.

use std::sync::Arc;

use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

use crate::auth::validate_auth_token;
use crate::config::ServiceConfig;
use crate::error::LockingError;
use crate::executor::LockExecutor;
use crate::logging::{generate_correlation_id, Logger};
use crate::proto::{
    locking_service_server::LockingService, Door, GetLockStateRequest, GetLockStateResponse,
    LockRequest, LockResponse, UnlockRequest, UnlockResponse,
};
use crate::publisher::{SignalWriter, StatePublisher};
use crate::state::LockState;
use crate::validator::{SafetyValidator, SignalReader};

/// Implementation of the LockingService gRPC service.
pub struct LockingServiceImpl<R: SignalReader, W: SignalWriter> {
    /// Lock executor for executing lock/unlock operations.
    executor: LockExecutor,
    /// Safety validator for checking constraints.
    validator: SafetyValidator<R>,
    /// State publisher for publishing to DATA_BROKER.
    publisher: StatePublisher<W>,
    /// Service configuration.
    config: ServiceConfig,
    /// Logger for structured logging.
    logger: Logger,
}

impl<R: SignalReader, W: SignalWriter> LockingServiceImpl<R, W> {
    /// Creates a new LockingServiceImpl.
    pub fn new(
        lock_state: Arc<RwLock<LockState>>,
        signal_reader: R,
        signal_writer: W,
        config: ServiceConfig,
    ) -> Self {
        let executor = LockExecutor::new(Arc::clone(&lock_state), config.execution_timeout);
        let validator = SafetyValidator::new(signal_reader, config.validation_timeout);
        let publisher = StatePublisher::new(
            signal_writer,
            config.publish_max_retries,
            config.publish_base_delay,
        );
        let logger = Logger::default();

        Self {
            executor,
            validator,
            publisher,
            config,
            logger,
        }
    }

    /// Returns a reference to the lock executor.
    pub fn executor(&self) -> &LockExecutor {
        &self.executor
    }

    /// Validates the auth token from a request.
    fn validate_auth(&self, token: &str) -> Result<(), LockingError> {
        validate_auth_token(token, &self.config.valid_tokens)
    }
}

#[tonic::async_trait]
impl<R: SignalReader + 'static, W: SignalWriter + 'static> LockingService
    for LockingServiceImpl<R, W>
{
    /// Handles Lock RPC requests.
    async fn lock(&self, request: Request<LockRequest>) -> Result<Response<LockResponse>, Status> {
        let req = request.into_inner();
        let correlation_id = generate_correlation_id();
        let command_id = req.command_id.clone();
        let door = Door::try_from(req.door).unwrap_or(Door::Unknown);

        // Log command received
        self.logger
            .log_command_received(&correlation_id, &command_id, "lock", door);

        // Execute with overall timeout
        let result = tokio::time::timeout(self.config.execution_timeout, async {
            // 1. Validate auth token
            if let Err(e) = self.validate_auth(&req.auth_token) {
                self.logger
                    .log_auth_validation(&correlation_id, &command_id, false);
                return Err(e);
            }
            self.logger
                .log_auth_validation(&correlation_id, &command_id, true);

            // 2. Validate safety constraints (door must be closed)
            if let Err(e) = self.validator.validate_lock(door).await {
                self.logger.log_safety_validation(
                    &correlation_id,
                    &command_id,
                    door,
                    false,
                    Some(&e.to_string()),
                );
                return Err(e);
            }
            self.logger
                .log_safety_validation(&correlation_id, &command_id, door, true, None);

            // 3. Execute lock operation
            let exec_result = self.executor.execute_lock(door, command_id.clone()).await?;
            self.logger
                .log_execution(&correlation_id, &command_id, door, "lock", true);

            // 4. Publish state (log if this fails, but don't fail the operation)
            let publish_result = self.publisher.publish_lock_state(door, true).await;
            self.logger.log_state_publish(
                &correlation_id,
                &command_id,
                door,
                true,
                publish_result.is_ok(),
            );

            Ok(exec_result)
        })
        .await;

        // Handle timeout or result
        match result {
            Ok(Ok(exec_result)) => {
                self.logger
                    .log_command_complete(&correlation_id, &command_id, true, None);
                Ok(Response::new(LockResponse {
                    success: true,
                    error_message: String::new(),
                    command_id: exec_result.command_id,
                }))
            }
            Ok(Err(e)) => {
                self.logger.log_command_complete(
                    &correlation_id,
                    &command_id,
                    false,
                    Some(&e.to_string()),
                );
                Err(e.into())
            }
            Err(_) => {
                let err =
                    LockingError::TimeoutError(self.config.execution_timeout.as_millis() as u64);
                self.logger.log_command_complete(
                    &correlation_id,
                    &command_id,
                    false,
                    Some(&err.to_string()),
                );
                Err(err.into())
            }
        }
    }

    /// Handles Unlock RPC requests.
    async fn unlock(
        &self,
        request: Request<UnlockRequest>,
    ) -> Result<Response<UnlockResponse>, Status> {
        let req = request.into_inner();
        let correlation_id = generate_correlation_id();
        let command_id = req.command_id.clone();
        let door = Door::try_from(req.door).unwrap_or(Door::Unknown);

        // Log command received
        self.logger
            .log_command_received(&correlation_id, &command_id, "unlock", door);

        // Execute with overall timeout
        let result = tokio::time::timeout(self.config.execution_timeout, async {
            // 1. Validate auth token
            if let Err(e) = self.validate_auth(&req.auth_token) {
                self.logger
                    .log_auth_validation(&correlation_id, &command_id, false);
                return Err(e);
            }
            self.logger
                .log_auth_validation(&correlation_id, &command_id, true);

            // 2. Validate safety constraints (vehicle must be stationary)
            if let Err(e) = self.validator.validate_unlock().await {
                self.logger.log_safety_validation(
                    &correlation_id,
                    &command_id,
                    door,
                    false,
                    Some(&e.to_string()),
                );
                return Err(e);
            }
            self.logger
                .log_safety_validation(&correlation_id, &command_id, door, true, None);

            // 3. Execute unlock operation
            let exec_result = self
                .executor
                .execute_unlock(door, command_id.clone())
                .await?;
            self.logger
                .log_execution(&correlation_id, &command_id, door, "unlock", true);

            // 4. Publish state (log if this fails, but don't fail the operation)
            let publish_result = self.publisher.publish_lock_state(door, false).await;
            self.logger.log_state_publish(
                &correlation_id,
                &command_id,
                door,
                false,
                publish_result.is_ok(),
            );

            Ok(exec_result)
        })
        .await;

        // Handle timeout or result
        match result {
            Ok(Ok(exec_result)) => {
                self.logger
                    .log_command_complete(&correlation_id, &command_id, true, None);
                Ok(Response::new(UnlockResponse {
                    success: true,
                    error_message: String::new(),
                    command_id: exec_result.command_id,
                }))
            }
            Ok(Err(e)) => {
                self.logger.log_command_complete(
                    &correlation_id,
                    &command_id,
                    false,
                    Some(&e.to_string()),
                );
                Err(e.into())
            }
            Err(_) => {
                let err =
                    LockingError::TimeoutError(self.config.execution_timeout.as_millis() as u64);
                self.logger.log_command_complete(
                    &correlation_id,
                    &command_id,
                    false,
                    Some(&err.to_string()),
                );
                Err(err.into())
            }
        }
    }

    /// Handles GetLockState RPC requests.
    async fn get_lock_state(
        &self,
        request: Request<GetLockStateRequest>,
    ) -> Result<Response<GetLockStateResponse>, Status> {
        let req = request.into_inner();
        let door = Door::try_from(req.door).unwrap_or(Door::Unknown);

        // Validate door
        if door == Door::Unknown {
            return Err(LockingError::InvalidDoor(door).into());
        }

        // Get state from executor
        let state = self.executor.lock_state();
        let lock_state = state.read().await;

        if let Some(door_state) = lock_state.get_door(door) {
            Ok(Response::new(GetLockStateResponse {
                door: door.into(),
                is_locked: door_state.is_locked,
                is_open: door_state.is_open,
            }))
        } else {
            Err(LockingError::InvalidDoor(door).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publisher::test_utils::MockSignalWriter;
    use crate::validator::test_utils::MockSignalReader;

    fn create_test_service() -> LockingServiceImpl<MockSignalReader, MockSignalWriter> {
        let state = Arc::new(RwLock::new(LockState::default()));
        let reader = MockSignalReader::new();
        let writer = MockSignalWriter::new();
        let config = ServiceConfig::default();

        // Set up default signals for safety validation
        reader.set_bool("Vehicle.Cabin.Door.Row1.DriverSide.IsOpen", false);
        reader.set_bool("Vehicle.Cabin.Door.Row1.PassengerSide.IsOpen", false);
        reader.set_bool("Vehicle.Cabin.Door.Row2.DriverSide.IsOpen", false);
        reader.set_bool("Vehicle.Cabin.Door.Row2.PassengerSide.IsOpen", false);
        reader.set_float("Vehicle.Speed", 0.0);

        LockingServiceImpl::new(state, reader, writer, config)
    }

    #[tokio::test]
    async fn test_lock_success() {
        let service = create_test_service();

        let request = Request::new(LockRequest {
            door: Door::Driver.into(),
            command_id: "cmd-1".to_string(),
            auth_token: "demo-token".to_string(),
        });

        let response = service.lock(request).await;

        assert!(response.is_ok());
        let resp = response.unwrap().into_inner();
        assert!(resp.success);
        assert_eq!(resp.command_id, "cmd-1");
    }

    #[tokio::test]
    async fn test_lock_invalid_auth() {
        let service = create_test_service();

        let request = Request::new(LockRequest {
            door: Door::Driver.into(),
            command_id: "cmd-1".to_string(),
            auth_token: "wrong-token".to_string(),
        });

        let response = service.lock(request).await;

        assert!(response.is_err());
        assert_eq!(response.unwrap_err().code(), tonic::Code::Unauthenticated);
    }

    #[tokio::test]
    async fn test_unlock_success() {
        let service = create_test_service();

        let request = Request::new(UnlockRequest {
            door: Door::Driver.into(),
            command_id: "cmd-1".to_string(),
            auth_token: "demo-token".to_string(),
        });

        let response = service.unlock(request).await;

        assert!(response.is_ok());
        let resp = response.unwrap().into_inner();
        assert!(resp.success);
    }

    #[tokio::test]
    async fn test_get_lock_state() {
        let service = create_test_service();

        // First lock a door
        let lock_request = Request::new(LockRequest {
            door: Door::Driver.into(),
            command_id: "cmd-1".to_string(),
            auth_token: "demo-token".to_string(),
        });
        service.lock(lock_request).await.unwrap();

        // Now get its state
        let request = Request::new(GetLockStateRequest {
            door: Door::Driver.into(),
        });

        let response = service.get_lock_state(request).await;

        assert!(response.is_ok());
        let resp = response.unwrap().into_inner();
        assert!(resp.is_locked);
        assert!(!resp.is_open);
    }

    #[tokio::test]
    async fn test_get_lock_state_invalid_door() {
        let service = create_test_service();

        let request = Request::new(GetLockStateRequest {
            door: Door::Unknown.into(),
        });

        let response = service.get_lock_state(request).await;

        assert!(response.is_err());
        assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
    }
}
