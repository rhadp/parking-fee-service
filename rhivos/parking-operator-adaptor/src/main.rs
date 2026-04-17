//! PARKING_OPERATOR_ADAPTOR — bridges PARKING_APP (gRPC) with a PARKING_OPERATOR backend (REST)
//! and autonomously manages parking sessions based on lock/unlock events from DATA_BROKER.
//!
//! Usage: parking-operator-adaptor serve

pub mod broker;
pub mod config;
pub mod event_loop;
pub mod grpc_server;
pub mod operator;
pub mod session;

#[cfg(test)]
pub mod proptest_cases;

use std::sync::Arc;

use tokio::sync::mpsc;
use tonic::transport::Server;
use tracing::{error, info, warn};

use broker::{BrokerClient, SIGNAL_IS_LOCKED, SIGNAL_SESSION_ACTIVE};
use config::load_config;
use event_loop::{manual_start, manual_stop, process_lock_event};
use grpc_server::{
    proto::parking_adaptor_server::ParkingAdaptorServer, ParkingAdaptorService, SessionCommand,
};
use operator::OperatorClient;
use session::Session;

/// Service version string sourced from `Cargo.toml`.
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_usage() {
    eprintln!(
        "Usage: parking-operator-adaptor serve

Environment:
  PARKING_OPERATOR_URL   Operator REST base URL (default: http://localhost:8080)
  DATA_BROKER_ADDR       DATA_BROKER gRPC address (default: http://localhost:55556)
  GRPC_PORT              gRPC listen port (default: 50053)
  VEHICLE_ID             Vehicle identifier (default: DEMO-VIN-001)
  ZONE_ID                Default parking zone (default: zone-demo-1)"
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    for arg in &args[1..] {
        if arg.starts_with('-') {
            print_usage();
            std::process::exit(1);
        }
    }

    match args.get(1).map(|s| s.as_str()) {
        Some("serve") => {
            let runtime =
                tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            let exit_code = runtime.block_on(run_service());
            std::process::exit(exit_code);
        }
        None => {
            println!("parking-operator-adaptor v{VERSION}");
        }
        Some(unknown) => {
            eprintln!("Unknown subcommand: {unknown}");
            print_usage();
            std::process::exit(1);
        }
    }
}

// ── Service entry point ───────────────────────────────────────────────────────

/// Run the service; returns the exit code (0 = success, 1 = fatal error).
async fn run_service() -> i32 {
    // Initialise structured logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load and validate configuration (08-REQ-7.*).
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            error!(?e, "Invalid configuration; exiting");
            return 1;
        }
    };

    // Log startup banner (08-REQ-8.1).
    info!(
        version = VERSION,
        parking_operator_url = %config.parking_operator_url,
        data_broker_addr = %config.data_broker_addr,
        grpc_port = config.grpc_port,
        vehicle_id = %config.vehicle_id,
        zone_id = %config.zone_id,
        "parking-operator-adaptor starting"
    );

    // Connect to DATA_BROKER with exponential-backoff retry (08-REQ-3.E3).
    let broker = match BrokerClient::connect(&config.data_broker_addr).await {
        Ok(b) => Arc::new(b),
        Err(e) => {
            error!(?e, "Failed to connect to DATA_BROKER after all retries; exiting");
            return 1;
        }
    };

    // Publish initial SessionActive=false (08-REQ-4.3).
    if let Err(e) = broker.set_bool(SIGNAL_SESSION_ACTIVE, false).await {
        warn!(?e, "Failed to publish initial SessionActive=false — continuing");
    }

    // Subscribe to IsLocked signal (08-REQ-3.2).
    let mut lock_rx = match broker.subscribe_bool(SIGNAL_IS_LOCKED).await {
        Ok(rx) => rx,
        Err(e) => {
            error!(?e, "Failed to subscribe to IsLocked signal; exiting");
            return 1;
        }
    };

    // Build operator client.
    let operator = Arc::new(OperatorClient::new(&config.parking_operator_url));

    // Create the event-loop command channel (8192 capacity is generous).
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<SessionCommand>(8192);

    // Spawn the event loop — sole owner of session state.
    let broker_for_loop = Arc::clone(&broker);
    let operator_for_loop = Arc::clone(&operator);
    let vehicle_id_loop = config.vehicle_id.clone();
    let zone_id_loop = config.zone_id.clone();

    let event_loop_handle = tokio::spawn(async move {
        let mut session = Session::new();
        loop {
            tokio::select! {
                // Process lock/unlock events (highest priority for autonomous behavior).
                is_locked_opt = lock_rx.recv() => {
                    match is_locked_opt {
                        Some(is_locked) => {
                            let _ = process_lock_event(
                                is_locked,
                                &mut session,
                                operator_for_loop.as_ref(),
                                broker_for_loop.as_ref(),
                                &vehicle_id_loop,
                                &zone_id_loop,
                            ).await;
                        }
                        None => {
                            warn!("Lock event channel closed; stopping event loop");
                            break;
                        }
                    }
                }

                // Process gRPC commands.
                cmd_opt = cmd_rx.recv() => {
                    match cmd_opt {
                        Some(cmd) => {
                            handle_command(
                                cmd,
                                &mut session,
                                operator_for_loop.as_ref(),
                                broker_for_loop.as_ref(),
                                &vehicle_id_loop,
                            ).await;
                        }
                        None => {
                            info!("Command channel closed; stopping event loop");
                            break;
                        }
                    }
                }
            }
        }
        info!("Event loop terminated");
    });

    // Prepare the gRPC server address.
    let grpc_addr = format!("0.0.0.0:{}", config.grpc_port)
        .parse()
        .expect("Valid socket address");

    let svc = ParkingAdaptorService::new(cmd_tx);

    // Set up shutdown signal watchers (08-REQ-8.3).
    let shutdown = setup_shutdown_signal();

    // Log "ready" message (08-REQ-8.2).
    info!(
        grpc_addr = %grpc_addr,
        "parking-operator-adaptor ready"
    );

    // Run gRPC server until shutdown signal.
    if let Err(e) = Server::builder()
        .add_service(ParkingAdaptorServer::new(svc))
        .serve_with_shutdown(grpc_addr, shutdown)
        .await
    {
        error!(?e, "gRPC server error");
        return 1;
    }

    // Wait for event loop to drain in-flight operations.
    let _ = event_loop_handle.await;

    info!("parking-operator-adaptor shut down cleanly");
    0
}

// ── Event loop command dispatch ───────────────────────────────────────────────

/// Dispatch a single `SessionCommand` inside the event loop.
///
/// Queries (`QueryStatus`, `QueryRate`) are answered synchronously from the
/// in-memory session without calling the operator or broker.
async fn handle_command(
    cmd: SessionCommand,
    session: &mut Session,
    operator: &impl operator::OperatorApi,
    publisher: &impl broker::SessionPublisher,
    vehicle_id: &str,
) {
    match cmd {
        SessionCommand::ManualStart { zone_id, reply } => {
            let result = manual_start(&zone_id, session, operator, publisher, vehicle_id).await;
            let _ = reply.send(result);
        }
        SessionCommand::ManualStop { reply } => {
            let result = manual_stop(session, operator, publisher).await;
            let _ = reply.send(result);
        }
        SessionCommand::QueryStatus { reply } => {
            let state = session.status().cloned();
            let _ = reply.send(state);
        }
        SessionCommand::QueryRate { reply } => {
            let rate = session.rate().cloned();
            let _ = reply.send(rate);
        }
    }
}

// ── Graceful shutdown ─────────────────────────────────────────────────────────

/// Returns a future that resolves when SIGTERM or SIGINT is received.
async fn setup_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
        let mut sigint =
            signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => { info!("Received SIGTERM — shutting down"); }
            _ = sigint.recv()  => { info!("Received SIGINT — shutting down"); }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to register Ctrl-C handler");
        info!("Received Ctrl-C — shutting down");
    }
}
