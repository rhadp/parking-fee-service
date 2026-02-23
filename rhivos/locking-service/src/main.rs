//! LOCKING_SERVICE entry point.
//!
//! Connects to DATA_BROKER via gRPC over Unix Domain Sockets and runs
//! the command processing loop. The UDS endpoint is configurable via
//! the `DATABROKER_UDS_PATH` environment variable.

use tracing_subscriber::EnvFilter;

/// Default UDS socket path for DATA_BROKER.
const DEFAULT_UDS_PATH: &str = "/tmp/kuksa-databroker.sock";

#[tokio::main]
async fn main() {
    // Initialize tracing with env filter (RUST_LOG)
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Resolve the DATA_BROKER endpoint
    let endpoint = resolve_endpoint();

    tracing::info!(endpoint = %endpoint, "starting locking-service");

    if let Err(e) = locking_service::service::run(&endpoint).await {
        tracing::error!(error = %e, "locking-service exited with error");
        std::process::exit(1);
    }
}

/// Resolve the DATA_BROKER endpoint from environment variables.
///
/// Priority:
/// 1. `DATABROKER_UDS_PATH` env var (formatted as `unix://` URI)
/// 2. Default UDS path: `/tmp/kuksa-databroker.sock`
fn resolve_endpoint() -> String {
    if let Ok(uds_path) = std::env::var("DATABROKER_UDS_PATH") {
        if uds_path.starts_with("unix://") {
            return uds_path;
        }
        return format!("unix://{uds_path}");
    }

    format!("unix://{DEFAULT_UDS_PATH}")
}
