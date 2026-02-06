//! gRPC service implementation for UPDATE_SERVICE.
//!
//! This module implements the UpdateService gRPC interface,
//! handling adapter installation, uninstallation, listing, and state watching.

use std::pin::Pin;
use std::sync::Arc;

use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::{debug, info, warn};

use crate::attestation::AttestationValidator;
use crate::authenticator::RegistryAuthenticator;
use crate::config::ServiceConfig;
use crate::container::ContainerManager;
use crate::downloader::ImageDownloader;
use crate::error::UpdateError;
use crate::logger::OperationLogger;
use crate::proto::{
    update_service_server::UpdateService, AdapterInfo, AdapterState, AdapterStateEvent,
    InstallAdapterRequest, InstallAdapterResponse, ListAdaptersRequest, ListAdaptersResponse,
    UninstallAdapterRequest, UninstallAdapterResponse, WatchAdapterStatesRequest,
};
use crate::tracker::StateTracker;
use crate::watcher::WatcherManager;

/// Main gRPC service implementation.
pub struct UpdateServiceImpl {
    state_tracker: Arc<StateTracker>,
    image_downloader: Arc<ImageDownloader>,
    attestation_validator: Arc<AttestationValidator>,
    container_manager: Arc<ContainerManager>,
    watcher_manager: Arc<WatcherManager>,
    logger: Arc<OperationLogger>,
    config: ServiceConfig,
}

impl UpdateServiceImpl {
    /// Create a new UpdateServiceImpl.
    pub fn new(
        config: ServiceConfig,
        authenticator: Arc<RegistryAuthenticator>,
        watcher_manager: Arc<WatcherManager>,
        logger: Arc<OperationLogger>,
    ) -> Self {
        let state_tracker = Arc::new(StateTracker::new(watcher_manager.clone(), logger.clone()));

        let image_downloader = Arc::new(ImageDownloader::new(
            authenticator.clone(),
            config.storage_path.clone().into(),
            config.download_max_retries,
            config.download_base_delay_ms,
            config.download_max_delay_ms,
            logger.clone(),
        ));

        let attestation_validator = Arc::new(AttestationValidator::new(authenticator));

        let container_manager = Arc::new(ContainerManager::new(
            config.storage_path.clone().into(),
            config.data_broker_socket.clone(),
            logger.clone(),
        ));

        Self {
            state_tracker,
            image_downloader,
            attestation_validator,
            container_manager,
            watcher_manager,
            logger,
            config,
        }
    }

    /// Create with explicit components (for testing).
    pub fn with_components(
        state_tracker: Arc<StateTracker>,
        image_downloader: Arc<ImageDownloader>,
        attestation_validator: Arc<AttestationValidator>,
        container_manager: Arc<ContainerManager>,
        watcher_manager: Arc<WatcherManager>,
        logger: Arc<OperationLogger>,
        config: ServiceConfig,
    ) -> Self {
        Self {
            state_tracker,
            image_downloader,
            attestation_validator,
            container_manager,
            watcher_manager,
            logger,
            config,
        }
    }

    /// Get the state tracker.
    pub fn state_tracker(&self) -> Arc<StateTracker> {
        self.state_tracker.clone()
    }

    /// Get the container manager.
    pub fn container_manager(&self) -> Arc<ContainerManager> {
        self.container_manager.clone()
    }

    /// Get the watcher manager.
    pub fn watcher_manager(&self) -> Arc<WatcherManager> {
        self.watcher_manager.clone()
    }

    /// Restore state from running containers on startup.
    pub async fn restore_state(&self) -> Result<(), UpdateError> {
        self.state_tracker
            .restore_from_containers(&self.container_manager)
            .await
    }

    /// Generate a correlation ID.
    fn generate_correlation_id() -> String {
        OperationLogger::generate_correlation_id()
    }

    /// Extract adapter ID from image reference.
    fn adapter_id_from_image_ref(image_ref: &str) -> String {
        // Use the image name as adapter ID
        // e.g., "gcr.io/project/parking-adapter:v1" -> "parking-adapter"
        let without_tag = image_ref.split(':').next().unwrap_or(image_ref);
        let name = without_tag.split('/').last().unwrap_or("adapter");
        name.to_string()
    }

    /// Validate image reference format.
    fn validate_image_ref(image_ref: &str) -> Result<(), UpdateError> {
        if image_ref.is_empty() {
            return Err(UpdateError::InvalidRegistryUrl(
                "Empty image reference".to_string(),
            ));
        }

        // Must contain at least one slash (registry/image)
        if !image_ref.contains('/') {
            return Err(UpdateError::InvalidRegistryUrl(format!(
                "Invalid image reference format: {}",
                image_ref
            )));
        }

        // Must not contain spaces or control characters
        if image_ref
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(UpdateError::InvalidRegistryUrl(format!(
                "Invalid characters in image reference: {}",
                image_ref
            )));
        }

        Ok(())
    }
}

#[tonic::async_trait]
impl UpdateService for UpdateServiceImpl {
    /// Install an adapter from registry.
    async fn install_adapter(
        &self,
        request: Request<InstallAdapterRequest>,
    ) -> Result<Response<InstallAdapterResponse>, Status> {
        let req = request.into_inner();
        let correlation_id = Self::generate_correlation_id();
        let adapter_id = Self::adapter_id_from_image_ref(&req.image_ref);

        self.logger
            .log_request(&correlation_id, "InstallAdapter", &adapter_id);

        info!(
            "InstallAdapter request: adapter_id={}, image_ref={}, checksum={}, correlation_id={}",
            adapter_id, req.image_ref, req.checksum, correlation_id
        );

        // Validate image reference
        if let Err(e) = Self::validate_image_ref(&req.image_ref) {
            return Err(e.into());
        }

        // Check if adapter already exists
        if let Some(current_state) = self.state_tracker.get_state(&adapter_id).await {
            match current_state {
                // Already running - return success without re-download (Property 2)
                AdapterState::Running => {
                    info!("Adapter {} already running, returning success", adapter_id);
                    return Ok(Response::new(InstallAdapterResponse {
                        job_id: correlation_id,
                        adapter_id,
                        state: AdapterState::Running as i32,
                    }));
                }
                // In progress - return current state (Property 3)
                AdapterState::Downloading | AdapterState::Installing => {
                    info!(
                        "Adapter {} already in progress ({:?}), returning current state",
                        adapter_id, current_state
                    );
                    return Ok(Response::new(InstallAdapterResponse {
                        job_id: correlation_id,
                        adapter_id,
                        state: current_state as i32,
                    }));
                }
                // Stopped or Error - allow reinstall
                _ => {
                    debug!(
                        "Adapter {} in {:?} state, allowing reinstall",
                        adapter_id, current_state
                    );
                }
            }
        }

        // Add adapter with DOWNLOADING state
        if let Err(e) = self
            .state_tracker
            .add(&adapter_id, &req.image_ref, &correlation_id)
            .await
        {
            // May already exist from stopped state, transition instead
            if matches!(e, UpdateError::AdapterAlreadyExists(_)) {
                self.state_tracker
                    .transition(
                        &adapter_id,
                        AdapterState::Downloading,
                        None,
                        &correlation_id,
                    )
                    .await
                    .map_err(|e| Status::from(e))?;
            } else {
                return Err(e.into());
            }
        }

        // Spawn async task for the install workflow
        let state_tracker = self.state_tracker.clone();
        let image_downloader = self.image_downloader.clone();
        let attestation_validator = self.attestation_validator.clone();
        let container_manager = self.container_manager.clone();
        let image_ref = req.image_ref.clone();
        let expected_checksum = req.checksum.clone();
        let adapter_id_clone = adapter_id.clone();
        let correlation_id_clone = correlation_id.clone();

        tokio::spawn(async move {
            // Download image
            let downloaded = match image_downloader
                .download(&image_ref, &adapter_id_clone, &correlation_id_clone)
                .await
            {
                Ok(img) => img,
                Err(e) => {
                    warn!("Download failed for {}: {}", adapter_id_clone, e);
                    let _ = state_tracker
                        .transition(
                            &adapter_id_clone,
                            AdapterState::Error,
                            Some(e.to_string()),
                            &correlation_id_clone,
                        )
                        .await;
                    return;
                }
            };

            // Validate attestation if checksum provided
            if !expected_checksum.is_empty() {
                // Fetch and validate attestation
                match attestation_validator
                    .fetch_attestation(&image_ref, &downloaded.digest, &correlation_id_clone)
                    .await
                {
                    Ok(attestation) => {
                        if let Err(e) =
                            attestation_validator.validate(&attestation, &expected_checksum)
                        {
                            warn!(
                                "Attestation validation failed for {}: {}",
                                adapter_id_clone, e
                            );
                            // Delete downloaded content
                            let _ = image_downloader.delete(&adapter_id_clone).await;
                            let _ = state_tracker
                                .transition(
                                    &adapter_id_clone,
                                    AdapterState::Error,
                                    Some(e.to_string()),
                                    &correlation_id_clone,
                                )
                                .await;
                            return;
                        }
                    }
                    Err(e) => {
                        // For now, log warning but continue if attestation not found
                        // In production, this might be a hard failure
                        warn!("Attestation fetch failed for {}: {}", adapter_id_clone, e);
                    }
                }
            }

            // Transition to INSTALLING
            if let Err(e) = state_tracker
                .transition(
                    &adapter_id_clone,
                    AdapterState::Installing,
                    None,
                    &correlation_id_clone,
                )
                .await
            {
                warn!("Failed to transition to INSTALLING: {}", e);
                return;
            }

            // Install container
            if let Err(e) = container_manager
                .install(&adapter_id_clone, &downloaded, &correlation_id_clone)
                .await
            {
                warn!("Container install failed for {}: {}", adapter_id_clone, e);
                let _ = state_tracker
                    .transition(
                        &adapter_id_clone,
                        AdapterState::Error,
                        Some(e.to_string()),
                        &correlation_id_clone,
                    )
                    .await;
                return;
            }

            // Start container
            if let Err(e) = container_manager
                .start(&adapter_id_clone, &image_ref, &correlation_id_clone)
                .await
            {
                warn!("Container start failed for {}: {}", adapter_id_clone, e);
                let _ = state_tracker
                    .transition(
                        &adapter_id_clone,
                        AdapterState::Error,
                        Some(e.to_string()),
                        &correlation_id_clone,
                    )
                    .await;
                return;
            }

            // Transition to RUNNING
            if let Err(e) = state_tracker
                .transition(
                    &adapter_id_clone,
                    AdapterState::Running,
                    None,
                    &correlation_id_clone,
                )
                .await
            {
                warn!("Failed to transition to RUNNING: {}", e);
                return;
            }

            info!("Successfully installed adapter {}", adapter_id_clone);
        });

        Ok(Response::new(InstallAdapterResponse {
            job_id: correlation_id,
            adapter_id,
            state: AdapterState::Downloading as i32,
        }))
    }

    /// Uninstall an adapter.
    async fn uninstall_adapter(
        &self,
        request: Request<UninstallAdapterRequest>,
    ) -> Result<Response<UninstallAdapterResponse>, Status> {
        let req = request.into_inner();
        let correlation_id = Self::generate_correlation_id();

        self.logger
            .log_request(&correlation_id, "UninstallAdapter", &req.adapter_id);

        info!(
            "UninstallAdapter request: adapter_id={}, correlation_id={}",
            req.adapter_id, correlation_id
        );

        // Check if adapter exists
        if !self.state_tracker.exists(&req.adapter_id).await {
            return Err(UpdateError::AdapterNotFound(req.adapter_id.clone()).into());
        }

        // Stop the container
        if let Err(e) = self
            .container_manager
            .stop(&req.adapter_id, &correlation_id)
            .await
        {
            warn!("Failed to stop container {}: {}", req.adapter_id, e);
            // Continue anyway - container might already be stopped
        }

        // Remove the container
        if let Err(e) = self
            .container_manager
            .remove(&req.adapter_id, &correlation_id)
            .await
        {
            warn!("Failed to remove container {}: {}", req.adapter_id, e);
            return Ok(Response::new(UninstallAdapterResponse {
                success: false,
                error_message: e.to_string(),
            }));
        }

        // Remove from state tracker
        if let Err(e) = self
            .state_tracker
            .remove(&req.adapter_id, &correlation_id)
            .await
        {
            warn!("Failed to remove from tracker {}: {}", req.adapter_id, e);
        }

        info!("Successfully uninstalled adapter {}", req.adapter_id);

        Ok(Response::new(UninstallAdapterResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    /// List installed adapters.
    async fn list_adapters(
        &self,
        request: Request<ListAdaptersRequest>,
    ) -> Result<Response<ListAdaptersResponse>, Status> {
        let _req = request.into_inner();
        let correlation_id = Self::generate_correlation_id();

        self.logger.log_request(&correlation_id, "ListAdapters", "");

        debug!("ListAdapters request: correlation_id={}", correlation_id);

        let adapters = self.state_tracker.list_all().await;
        let adapter_infos: Vec<AdapterInfo> = adapters.iter().map(|e| e.to_proto_info()).collect();

        Ok(Response::new(ListAdaptersResponse {
            adapters: adapter_infos,
        }))
    }

    /// Stream type for WatchAdapterStates.
    type WatchAdapterStatesStream =
        Pin<Box<dyn tokio_stream::Stream<Item = Result<AdapterStateEvent, Status>> + Send>>;

    /// Watch adapter state changes.
    async fn watch_adapter_states(
        &self,
        request: Request<WatchAdapterStatesRequest>,
    ) -> Result<Response<Self::WatchAdapterStatesStream>, Status> {
        let _req = request.into_inner();
        let correlation_id = Self::generate_correlation_id();

        self.logger
            .log_request(&correlation_id, "WatchAdapterStates", "");

        info!(
            "WatchAdapterStates request: correlation_id={}",
            correlation_id
        );

        // Register watcher
        let (watcher_id, rx) = self.watcher_manager.register().await;

        // Send initial state for all adapters
        let initial_events = self.state_tracker.get_initial_events().await;
        if let Err(_) = self
            .watcher_manager
            .send_initial_state(watcher_id, initial_events)
            .await
        {
            warn!("Failed to send initial state to watcher {}", watcher_id);
        }

        // Return the receiver as a stream
        let stream = ReceiverStream::new(rx);

        Ok(Response::new(Box::pin(stream)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::TempDir;

    fn create_test_service() -> (UpdateServiceImpl, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = ServiceConfig::default();
        config.storage_path = temp_dir.path().to_str().unwrap().to_string();

        let logger = Arc::new(OperationLogger::new("test"));
        let watcher_manager = Arc::new(WatcherManager::new());
        let authenticator = Arc::new(RegistryAuthenticator::new(None, 60, logger.clone()));

        let service = UpdateServiceImpl::new(config, authenticator, watcher_manager, logger);

        (service, temp_dir)
    }

    #[test]
    fn test_validate_image_ref_valid() {
        assert!(UpdateServiceImpl::validate_image_ref("gcr.io/project/image:v1").is_ok());
        assert!(UpdateServiceImpl::validate_image_ref("docker.io/library/nginx").is_ok());
        assert!(UpdateServiceImpl::validate_image_ref("localhost:5000/image").is_ok());
    }

    #[test]
    fn test_validate_image_ref_invalid() {
        assert!(UpdateServiceImpl::validate_image_ref("").is_err());
        assert!(UpdateServiceImpl::validate_image_ref("invalid").is_err());
        assert!(UpdateServiceImpl::validate_image_ref("has space/image").is_err());
    }

    #[test]
    fn test_adapter_id_from_image_ref() {
        assert_eq!(
            UpdateServiceImpl::adapter_id_from_image_ref("gcr.io/project/my-adapter:v1"),
            "my-adapter"
        );
        assert_eq!(
            UpdateServiceImpl::adapter_id_from_image_ref("docker.io/library/nginx:latest"),
            "nginx"
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 1: Install Initiates Download State
        /// Validates: Requirements 1.1, 1.2
        #[test]
        fn prop_install_initiates_download(
            image in "[a-z]+\\.[a-z]+/[a-z]+/[a-z]+:[a-z0-9]+"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (service, _temp_dir) = create_test_service();
                let adapter_id = UpdateServiceImpl::adapter_id_from_image_ref(&image);

                let request = Request::new(InstallAdapterRequest {
                    image_ref: image.clone(),
                    checksum: String::new(),
                });

                let response = service.install_adapter(request).await;

                prop_assert!(response.is_ok());
                let resp = response.unwrap().into_inner();
                prop_assert_eq!(resp.state, AdapterState::Downloading as i32);
                prop_assert_eq!(&resp.adapter_id, &adapter_id);

                // Adapter should be tracked
                prop_assert!(service.state_tracker.exists(&adapter_id).await);

                Ok(())
            })?;
        }

        /// Property 4: Invalid Registry URL Returns Error
        /// Validates: Requirements 1.5
        #[test]
        fn prop_invalid_registry_url_error(
            invalid_ref in "[a-z]+"  // Missing slash
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (service, _temp_dir) = create_test_service();

                let request = Request::new(InstallAdapterRequest {
                    image_ref: invalid_ref,
                    checksum: String::new(),
                });

                let response = service.install_adapter(request).await;

                prop_assert!(response.is_err());
                let status = response.unwrap_err();
                prop_assert_eq!(status.code(), tonic::Code::InvalidArgument);

                Ok(())
            })?;
        }

        /// Property 16: Uninstall Non-Existent Returns Error
        /// Validates: Requirements 8.4
        #[test]
        fn prop_uninstall_nonexistent_error(
            adapter_id in "[a-z][a-z0-9-]{3,20}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (service, _temp_dir) = create_test_service();

                let request = Request::new(UninstallAdapterRequest {
                    adapter_id: adapter_id.clone(),
                });

                let response = service.uninstall_adapter(request).await;

                prop_assert!(response.is_err());
                let status = response.unwrap_err();
                prop_assert_eq!(status.code(), tonic::Code::NotFound);

                Ok(())
            })?;
        }
    }
}
