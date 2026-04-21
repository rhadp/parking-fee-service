use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::Mutex;
use tonic::transport::Server;
use tracing::{error, info, warn};

use parking_operator_adaptor::broker::{BrokerClient, SessionPublisher};
use parking_operator_adaptor::config::load_config;
use parking_operator_adaptor::event_loop::process_lock_event;
use parking_operator_adaptor::grpc_server::{ParkingAdaptorServer, ParkingAdaptorService};
use parking_operator_adaptor::operator::OperatorClient;
use parking_operator_adaptor::session::Session;

/// Service version string logged on startup.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Entry point.
///
/// Requires the `serve` subcommand to start the service. Without it (or with
/// unknown flags) the binary prints usage and exits 0 / 1.
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Require the "serve" subcommand (01-REQ-4.1).
    // No args → print version (01-REQ-4.1); unknown flags → exit 1 (01-REQ-4.E1).
    if args.get(1).map(String::as_str) != Some("serve") {
        for arg in &args[1..] {
            if arg.starts_with('-') {
                eprintln!("usage: parking-operator-adaptor serve");
                std::process::exit(1);
            }
        }
        println!("parking-operator-adaptor v0.1.0");
        return;
    }

    // Initialise structured logging (level controlled by RUST_LOG env var).
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // ── Step 1: Load configuration ────────────────────────────────────────────

    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ERROR: {e}");
            std::process::exit(1);
        }
    };

    // 08-REQ-8.1: Log version and all configuration values on startup.
    info!(
        version = VERSION,
        parking_operator_url = %config.parking_operator_url,
        data_broker_addr = %config.data_broker_addr,
        grpc_port = config.grpc_port,
        vehicle_id = %config.vehicle_id,
        zone_id = %config.zone_id,
        "parking-operator-adaptor starting"
    );

    // ── Step 2: Connect to DATA_BROKER with exponential-backoff retry ─────────

    let broker = match BrokerClient::connect(&config.data_broker_addr).await {
        Ok(b) => b,
        Err(e) => {
            error!(error = %e, "Failed to connect to DATA_BROKER after retries; exiting");
            std::process::exit(1);
        }
    };

    // 08-REQ-4.3: Publish initial SessionActive=false.
    if let Err(e) = broker.set_session_active(false).await {
        warn!(error = %e, "Failed to publish initial SessionActive=false; continuing");
    }

    // ── Step 3: Subscribe to IsLocked signal ──────────────────────────────────

    let mut is_locked_rx = match broker.subscribe_is_locked().await {
        Ok(rx) => rx,
        Err(e) => {
            error!(error = %e, "Failed to subscribe to IsLocked signal; exiting");
            std::process::exit(1);
        }
    };

    // ── Step 4: Initialise shared state ──────────────────────────────────────

    let session = Arc::new(Mutex::new(Session::new()));
    let operator: Arc<dyn parking_operator_adaptor::operator::OperatorApi> =
        Arc::new(OperatorClient::new(&config.parking_operator_url));
    let publisher: Arc<dyn SessionPublisher> = Arc::new(broker);

    // ── Step 5: Spawn lock-event processing task ──────────────────────────────

    let session_event = session.clone();
    let operator_event = operator.clone();
    let publisher_event = publisher.clone();
    let vehicle_id_event = config.vehicle_id.clone();
    let zone_id_event = config.zone_id.clone();

    tokio::spawn(async move {
        while let Some(is_locked) = is_locked_rx.recv().await {
            let mut sess = session_event.lock().await;
            let result = process_lock_event(
                is_locked,
                &mut sess,
                operator_event.as_ref(),
                publisher_event.as_ref(),
                &vehicle_id_event,
                &zone_id_event,
            )
            .await;

            if let Err(e) = result {
                error!(is_locked, error = %e, "Lock event processing failed");
            }
        }
        warn!("IsLocked subscription stream ended");
    });

    // ── Step 6: Build gRPC server ─────────────────────────────────────────────

    let svc = ParkingAdaptorService::new(
        session,
        operator,
        publisher,
        config.vehicle_id.clone(),
        config.zone_id.clone(),
    );

    let addr: SocketAddr = format!("0.0.0.0:{}", config.grpc_port)
        .parse()
        .expect("invalid gRPC port");

    // 08-REQ-8.2: Log "ready" message once fully initialised.
    info!(addr = %addr, "parking-operator-adaptor ready");

    // ── Step 7: Serve with graceful shutdown ─────────────────────────────────

    // 08-REQ-8.3: Handle SIGTERM/SIGINT and exit with code 0.
    let server = Server::builder()
        .add_service(ParkingAdaptorServer::new(svc))
        .serve_with_shutdown(addr, async {
            shutdown_signal().await;
        });

    if let Err(e) = server.await {
        error!(error = %e, "gRPC server error");
        std::process::exit(1);
    }

    info!("parking-operator-adaptor shut down cleanly");
}

/// Wait for SIGTERM or SIGINT (Ctrl-C).
///
/// Both signals cause a clean shutdown (08-REQ-8.3).
async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let sigterm = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { info!("Received SIGINT, shutting down"); }
        _ = sigterm => { info!("Received SIGTERM, shutting down"); }
    }
}
