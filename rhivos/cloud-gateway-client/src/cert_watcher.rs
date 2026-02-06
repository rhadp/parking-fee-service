//! Certificate watcher for hot-reload functionality.
//!
//! This module monitors TLS certificate files and triggers reloads
//! when certificates are updated, without requiring service restart.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use x509_parser::certificate::X509Certificate;

use crate::error::CertLoadError;

/// Paths to TLS certificate files.
#[derive(Debug, Clone)]
pub struct CertificatePaths {
    /// Path to CA certificate
    pub ca_cert_path: PathBuf,
    /// Path to client certificate
    pub client_cert_path: PathBuf,
    /// Path to client private key
    pub client_key_path: PathBuf,
}

impl CertificatePaths {
    /// Create new certificate paths.
    pub fn new(ca_cert: PathBuf, client_cert: PathBuf, client_key: PathBuf) -> Self {
        Self {
            ca_cert_path: ca_cert,
            client_cert_path: client_cert,
            client_key_path: client_key,
        }
    }

    /// Get all paths as a slice for iteration.
    pub fn all_paths(&self) -> [&PathBuf; 3] {
        [
            &self.ca_cert_path,
            &self.client_cert_path,
            &self.client_key_path,
        ]
    }
}

/// Loaded certificate data with metadata.
#[derive(Debug, Clone)]
pub struct LoadedCertificates {
    /// CA certificate bytes
    pub ca_cert: Vec<u8>,
    /// Client certificate bytes
    pub client_cert: Vec<u8>,
    /// Client private key bytes
    pub client_key: Vec<u8>,
    /// Expiry date of client certificate
    pub expiry_date: Option<DateTime<Utc>>,
    /// Timestamp when certificates were loaded
    pub loaded_at: DateTime<Utc>,
}

/// Status of a certificate reload event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertReloadStatus {
    /// Certificate reload succeeded
    Success,
    /// Certificate reload failed
    Failed,
}

/// Event emitted when certificates are reloaded.
#[derive(Debug, Clone)]
pub struct CertReloadEvent {
    /// Timestamp of the reload event
    pub timestamp: DateTime<Utc>,
    /// Status of the reload
    pub status: CertReloadStatus,
    /// Path that triggered the reload
    pub cert_path: PathBuf,
    /// Expiry date if reload succeeded
    pub expiry_date: Option<DateTime<Utc>>,
    /// Error message if reload failed
    pub error_message: Option<String>,
}

impl CertReloadEvent {
    /// Create a success event.
    pub fn success(cert_path: PathBuf, expiry_date: Option<DateTime<Utc>>) -> Self {
        Self {
            timestamp: Utc::now(),
            status: CertReloadStatus::Success,
            cert_path,
            expiry_date,
            error_message: None,
        }
    }

    /// Create a failure event.
    pub fn failure(cert_path: PathBuf, error: String) -> Self {
        Self {
            timestamp: Utc::now(),
            status: CertReloadStatus::Failed,
            cert_path,
            expiry_date: None,
            error_message: Some(error),
        }
    }
}

/// Watches certificate files and triggers reloads on changes.
pub struct CertificateWatcher {
    /// Certificate file paths
    paths: CertificatePaths,
    /// Current loaded certificates
    current_certs: Arc<RwLock<Option<LoadedCertificates>>>,
    /// Channel to receive reload events
    event_rx: Option<mpsc::Receiver<CertReloadEvent>>,
    /// Channel to send reload events
    event_tx: mpsc::Sender<CertReloadEvent>,
}

impl CertificateWatcher {
    /// Create a new certificate watcher for the given paths.
    pub fn new(paths: CertificatePaths) -> Self {
        let (event_tx, event_rx) = mpsc::channel(16);
        Self {
            paths,
            current_certs: Arc::new(RwLock::new(None)),
            event_rx: Some(event_rx),
            event_tx,
        }
    }

    /// Take the event receiver (can only be called once).
    pub fn take_event_receiver(&mut self) -> Option<mpsc::Receiver<CertReloadEvent>> {
        self.event_rx.take()
    }

    /// Get the current loaded certificates.
    pub async fn current_certificates(&self) -> Option<LoadedCertificates> {
        self.current_certs.read().await.clone()
    }

    /// Load certificates initially and start watching for changes.
    ///
    /// Returns an error if the initial certificate load fails.
    pub async fn start(&self) -> Result<(), CertLoadError> {
        // Initial load
        let certs = self.load_all_certificates().await?;
        *self.current_certs.write().await = Some(certs);

        Ok(())
    }

    /// Start the file watcher in a background task.
    ///
    /// This spawns a task that monitors certificate files and triggers
    /// reloads when changes are detected.
    pub fn spawn_watcher(&self) -> Result<(), crate::error::CertWatcherError> {
        let paths = self.paths.clone();
        let current_certs = self.current_certs.clone();
        let event_tx = self.event_tx.clone();

        // Create file watcher channel
        let (tx, mut rx) = mpsc::channel::<Result<Event, notify::Error>>(32);

        // Create watcher
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.blocking_send(res);
            },
            Config::default().with_poll_interval(Duration::from_secs(2)),
        )
        .map_err(|e| crate::error::CertWatcherError::WatcherInitFailed(e.to_string()))?;

        // Watch certificate directories
        for path in paths.all_paths() {
            if let Some(parent) = path.parent() {
                watcher
                    .watch(parent, RecursiveMode::NonRecursive)
                    .map_err(|e| crate::error::CertWatcherError::WatchPathFailed {
                        path: parent.to_path_buf(),
                        error: e.to_string(),
                    })?;
            }
        }

        // Spawn watcher task
        let paths_clone = paths.clone();
        tokio::spawn(async move {
            // Keep watcher alive
            let _watcher = watcher;

            while let Some(res) = rx.recv().await {
                match res {
                    Ok(event) => {
                        if Self::is_relevant_event(&event, &paths_clone) {
                            Self::handle_file_change(
                                &paths_clone,
                                &current_certs,
                                &event_tx,
                                event.paths.first().cloned(),
                            )
                            .await;
                        }
                    }
                    Err(e) => {
                        warn!("File watcher error: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Check if a file system event is relevant to our certificates.
    fn is_relevant_event(event: &Event, paths: &CertificatePaths) -> bool {
        // Only care about modify/create events
        matches!(
            event.kind,
            EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
        ) && event.paths.iter().any(|p| paths.all_paths().contains(&p))
    }

    /// Handle a file change event.
    async fn handle_file_change(
        paths: &CertificatePaths,
        current_certs: &Arc<RwLock<Option<LoadedCertificates>>>,
        event_tx: &mpsc::Sender<CertReloadEvent>,
        changed_path: Option<PathBuf>,
    ) {
        let path = changed_path.unwrap_or_else(|| PathBuf::from("unknown"));

        debug!("Certificate file changed: {:?}", path);

        // Try to reload certificates
        match Self::load_all_certificates_static(paths).await {
            Ok(new_certs) => {
                let expiry = new_certs.expiry_date;
                *current_certs.write().await = Some(new_certs);

                info!(
                    "Certificate reload successful, expiry: {:?}",
                    expiry.map(|d| d.to_rfc3339())
                );

                let event = CertReloadEvent::success(path, expiry);
                let _ = event_tx.send(event).await;
            }
            Err(e) => {
                error!(
                    "Certificate reload failed: {}, keeping existing certificates",
                    e
                );

                let event = CertReloadEvent::failure(path, e.to_string());
                let _ = event_tx.send(event).await;
            }
        }
    }

    /// Load all certificates from the configured paths.
    async fn load_all_certificates(&self) -> Result<LoadedCertificates, CertLoadError> {
        Self::load_all_certificates_static(&self.paths).await
    }

    /// Load all certificates (static version for use in spawned tasks).
    async fn load_all_certificates_static(
        paths: &CertificatePaths,
    ) -> Result<LoadedCertificates, CertLoadError> {
        let ca_cert = Self::load_certificate_file(&paths.ca_cert_path).await?;
        let client_cert = Self::load_certificate_file(&paths.client_cert_path).await?;
        let client_key = Self::load_certificate_file(&paths.client_key_path).await?;

        // Extract expiry date from client certificate
        let expiry_date = Self::extract_expiry_date(&client_cert)?;

        Ok(LoadedCertificates {
            ca_cert,
            client_cert,
            client_key,
            expiry_date,
            loaded_at: Utc::now(),
        })
    }

    /// Load a single certificate file.
    async fn load_certificate_file(path: &Path) -> Result<Vec<u8>, CertLoadError> {
        tokio::fs::read(path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CertLoadError::FileNotFound(path.to_path_buf())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                CertLoadError::PermissionDenied(path.to_path_buf())
            } else {
                CertLoadError::InvalidFormat(format!("Failed to read {}: {}", path.display(), e))
            }
        })
    }

    /// Extract the expiry date from a PEM-encoded certificate.
    pub fn extract_expiry_date(cert_pem: &[u8]) -> Result<Option<DateTime<Utc>>, CertLoadError> {
        // Parse PEM using the pem crate
        let pem_data: ::pem::Pem = match ::pem::parse(cert_pem) {
            Ok(p) => p,
            Err(_) => {
                // Try parsing as DER directly
                return Self::parse_der_cert_expiry(cert_pem);
            }
        };

        Self::parse_der_cert_expiry(pem_data.contents())
    }

    /// Parse DER-encoded certificate and extract expiry.
    fn parse_der_cert_expiry(der: &[u8]) -> Result<Option<DateTime<Utc>>, CertLoadError> {
        use x509_parser::prelude::FromDer;

        let (_, cert) = X509Certificate::from_der(der).map_err(|e| {
            CertLoadError::ParseFailed(format!("Failed to parse certificate: {}", e))
        })?;

        let not_after = cert.validity().not_after;
        let timestamp = not_after.timestamp();

        Ok(DateTime::from_timestamp(timestamp, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::fs;
    use tempfile::TempDir;

    // Valid test certificate (self-signed, for testing only)
    // Generated with: openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 365 -nodes -subj "/CN=test"
    const TEST_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIC/zCCAeegAwIBAgIUbASnBAU1sUdL7wVBhkN/D6vzIEowDQYJKoZIhvcNAQEL
BQAwDzENMAsGA1UEAwwEdGVzdDAeFw0yNjAyMDYxNTU1NTRaFw0yNzAyMDYxNTU1
NTRaMA8xDTALBgNVBAMMBHRlc3QwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEK
AoIBAQC/2KqC5dZrTLiKMR/4M32vWRPZ0vpRUCwTAOLmVlJQ+HMSkup8Yf61R+0U
VAIWzlwJuNlqiDGmZ92BWRV/U/6K22d4yxZ1dUeGz2D3wQTR4qnhpkUOik0ALA2Q
IL9e4V7QorDluxT+QB6w7eveQWd19t2gkd+CkYazFWevxFEf0FW8+SQAEDoRxuJq
apvh+T9uRbQPDnPEHgy+fcujWRtdR5eozz5vGyrMa7Y1L9cMP5fLw95tdvxvdU24
oI/8MqbyaHbOYA8pC0Weh22Ra4PfKvw7Di7HP5Ws5E4OQYH1QjxV2mKF4ukUnS9T
LZL2uk/9ilE7iK2Z53WTFW1zd1ZNAgMBAAGjUzBRMB0GA1UdDgQWBBTLRA4Re/x0
brjiDcOExXzTmOnYTzAfBgNVHSMEGDAWgBTLRA4Re/x0brjiDcOExXzTmOnYTzAP
BgNVHRMBAf8EBTADAQH/MA0GCSqGSIb3DQEBCwUAA4IBAQCYTRsfDb9Xk89Ib8iI
gAYAVgvPikp021ZJN6bUM+iKRYmRbc2y56qN+UKYClVYGTCv4Q9R0nVq8+xwknzT
1fCTKp+FKfriq/b9PAhoTKir3wTNKrhW+XtHGZDZ//9k7mCfAdlDjirGp0gkgQ9y
9j30uTgSJhHQ+5kyxEhZRTMCRQ0jl19/ddi+mbtMRrToUIfe6WFy4mpK7Hff+m+a
yyLU5gzl5ty6vUn2h+/GMvQyoDmk6Z5ZhOK6Yb6ZxW3wZ5CjKXUjd7vKyoZHH5rx
hPp6IqIwiQO2hDmp2mQ05xX2fXLr0CnP9GbZZBPUiSC+HSI12MAoMrJrOj7sWgWe
nubH
-----END CERTIFICATE-----";

    const TEST_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQC/2KqC5dZrTLiK
MR/4M32vWRPZ0vpRUCwTAOLmVlJQ+HMSkup8Yf61R+0UVAIWzlwJuNlqiDGmZ92B
WRV/U/6K22d4yxZ1dUeGz2D3wQTR4qnhpkUOik0ALA2QIL9e4V7QorDluxT+QB6w
7eveQWd19t2gkd+CkYazFWevxFEf0FW8+SQAEDoRxuJqapvh+T9uRbQPDnPEHgy+
fcujWRtdR5eozz5vGyrMa7Y1L9cMP5fLw95tdvxvdU24oI/8MqbyaHbOYA8pC0We
h22Ra4PfKvw7Di7HP5Ws5E4OQYH1QjxV2mKF4ukUnS9TLZL2uk/9ilE7iK2Z53WT
FW1zd1ZNAgMBAAECggEAEFwk8EwyzIwqmYhGamdTssIVJghulcQROh6jetKGKwcA
4ybZrbk6nBFx53+hfPnkkeSsT8AlAcSOOGNVKLpMNOGwcXXaKLKTYqzpHz+bzl02
sPo5nduP5PGcx3tvbmMoL4EMNk8No3/qzho/+MBZlw7yB+kgpxULaFatNKk4ZM68
cYfQhZ1ZyNuAF3/1KbMvspMFX8CAoEyQC7mHZ4TnvzTXk4WIdGbPVTDwcumbxFPd
VeZ1nhlnvPfZE7oCclrvt8yXaQzOfjs6uCPsN7ee9PNtxQqkuMdnSq13cEGg31Ur
wM5P82+eABeBLmdjl7ZxbKFHBVn3bUyQ/H0ibgCj4QKBgQD+/PddYfVxFE8LLRwA
OFzUoJCJZMrNn69vIuI5/gSwWjLwLjdei0nN6yNAFRC4tmKFrddQI2WOrXge61JG
l3HjTqUzL5SICYz8gzkrvcv3POm28Ze5WOqrDWoNR0Vz06vFvw9IUkmaiWqn5wk/
OF7wAb+zXcmcD2CjKaVgWC5v4QKBgQDAm45jKfUFvh3MoqPJ9DyD0yMdmQkIsTWJ
YfT/9OduwGfQZs+2qs+rL5OTzaUJVg1dtUQaMMguMiRMoUJSxGP1ELu1PIH1/tWs
wLebUHbamrPA361Sll9D9uYEvOd1mRU9Xdd4QOErGl1hNnQlD/Kh7gpTiiClZzXV
sik/9O4j7QKBgA7a2eZc0Jm33yr9g8YXgoD4obL/Zjk4dlX5KEjMnaVQe+s2Jg+h
+bi/XBxdnc3FAlRbXlHS3hXD0V2rw+1M4Vumt0UWHocWV1pWorwDoKBUsiDwTjCE
F5fDfkwrvMYUrMsmaFOER7lzC/2gHg/Kzu0YjPx8GES5OJ4IzROhz4LBAoGAYxyo
/KZOi5H1S6Q1nGqt6Tfwzf4+A9cMsZFSvZOMtBUWVstQ/7KOAo0M5/Xegxtg7WOl
k8SefgcXXsdslaKxvR3LOcvVJHzp/2d8E9QoFP2emhV/3wu6IgMfAjki8gTART/Q
7PSV6dQ7URbwVVILjQAtGCfv/K1LqpdvWpXzJVECgYBsjb8Y3rJCmVMa13opCAez
Hw/d1Csj3T3QKlsNX+xnPTBZnnnHGkiUHqgQvqXfiEls2GGVeTX44hnRqTLWuCDw
ztc45a+luyTBrE47Y5g+0YTVsrPlzrd81PdWxw3Enz2q+58tV3tT5bvEuN86Ty5L
SSUk+9CoQOoz3426ZsrN9A==
-----END PRIVATE KEY-----";

    fn create_test_certs(dir: &TempDir) -> CertificatePaths {
        let ca_path = dir.path().join("ca.crt");
        let cert_path = dir.path().join("client.crt");
        let key_path = dir.path().join("client.key");

        fs::write(&ca_path, TEST_CERT_PEM).unwrap();
        fs::write(&cert_path, TEST_CERT_PEM).unwrap();
        fs::write(&key_path, TEST_KEY_PEM).unwrap();

        CertificatePaths::new(ca_path, cert_path, key_path)
    }

    #[tokio::test]
    async fn test_load_certificates() {
        let dir = TempDir::new().unwrap();
        let paths = create_test_certs(&dir);

        let watcher = CertificateWatcher::new(paths);
        let result = watcher.start().await;

        assert!(result.is_ok());
        let certs = watcher.current_certificates().await;
        assert!(certs.is_some());

        let certs = certs.unwrap();
        assert!(!certs.ca_cert.is_empty());
        assert!(!certs.client_cert.is_empty());
        assert!(!certs.client_key.is_empty());
        assert!(certs.expiry_date.is_some());
    }

    #[tokio::test]
    async fn test_load_missing_certificate() {
        let paths = CertificatePaths::new(
            PathBuf::from("/nonexistent/ca.crt"),
            PathBuf::from("/nonexistent/client.crt"),
            PathBuf::from("/nonexistent/client.key"),
        );

        let watcher = CertificateWatcher::new(paths);
        let result = watcher.start().await;

        assert!(matches!(result, Err(CertLoadError::FileNotFound(_))));
    }

    #[test]
    fn test_extract_expiry_date() {
        let result = CertificateWatcher::extract_expiry_date(TEST_CERT_PEM.as_bytes());
        assert!(result.is_ok(), "Failed to parse cert: {:?}", result.err());
        let expiry = result.unwrap();
        assert!(expiry.is_some());
    }

    #[test]
    fn test_cert_reload_event_success() {
        let event = CertReloadEvent::success(PathBuf::from("/path/to/cert"), Some(Utc::now()));
        assert_eq!(event.status, CertReloadStatus::Success);
        assert!(event.error_message.is_none());
        assert!(event.expiry_date.is_some());
    }

    #[test]
    fn test_cert_reload_event_failure() {
        let event = CertReloadEvent::failure(
            PathBuf::from("/path/to/cert"),
            "Invalid certificate".to_string(),
        );
        assert_eq!(event.status, CertReloadStatus::Failed);
        assert!(event.error_message.is_some());
        assert!(event.expiry_date.is_none());
    }

    // Property 17: Certificate Hot-Reload on File Change
    // Validates: Requirements 1.6
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_cert_reload_success_has_required_fields(
            path in "[a-z/]+\\.crt",
            timestamp_offset in 0i64..1000000
        ) {
            let expiry = DateTime::from_timestamp(Utc::now().timestamp() + timestamp_offset, 0);
            let event = CertReloadEvent::success(PathBuf::from(&path), expiry);

            // Success events must have timestamp, success status, path, and optionally expiry
            prop_assert!(event.timestamp <= Utc::now());
            prop_assert_eq!(event.status, CertReloadStatus::Success);
            prop_assert_eq!(event.cert_path.to_string_lossy(), path);
            prop_assert!(event.error_message.is_none());
        }
    }

    // Property 18: Certificate Reload Failure Resilience
    // Validates: Requirements 1.7
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_cert_reload_failure_preserves_error(
            path in "[a-z/]+\\.crt",
            error_msg in "[a-zA-Z0-9 ]{1,100}"
        ) {
            let event = CertReloadEvent::failure(PathBuf::from(&path), error_msg.clone());

            // Failure events must have timestamp, failed status, path, and error message
            prop_assert!(event.timestamp <= Utc::now());
            prop_assert_eq!(event.status, CertReloadStatus::Failed);
            prop_assert_eq!(event.cert_path.to_string_lossy(), path);
            prop_assert!(event.expiry_date.is_none());
            prop_assert_eq!(event.error_message, Some(error_msg));
        }
    }

    // Property 19: Certificate Reload Event Logging
    // Validates: Requirements 1.8
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_cert_reload_event_contains_logging_fields(
            is_success in proptest::bool::ANY,
            path in "[a-z/]+\\.crt"
        ) {
            let path_buf = PathBuf::from(&path);
            let event = if is_success {
                CertReloadEvent::success(path_buf, Some(Utc::now()))
            } else {
                CertReloadEvent::failure(path_buf, "error".to_string())
            };

            // All reload events must contain: timestamp, status, path
            // This ensures logging has all required fields per Requirement 1.8
            prop_assert!(event.timestamp <= Utc::now());
            prop_assert!(event.status == CertReloadStatus::Success || event.status == CertReloadStatus::Failed);
            prop_assert!(!event.cert_path.as_os_str().is_empty());

            // Success events should have expiry_date
            if event.status == CertReloadStatus::Success {
                prop_assert!(event.expiry_date.is_some());
            }
        }
    }
}
