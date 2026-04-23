pub mod broker;
pub mod config;
pub mod event_loop;
pub mod grpc_server;
pub mod operator;
pub mod session;
#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod proptest_cases;

use broker::parking_adaptor::v1::parking_adaptor_server::ParkingAdaptorServer;
use broker::{BrokerClient, GrpcBrokerClient, SIGNAL_IS_LOCKED, SIGNAL_SESSION_ACTIVE};
use event_loop::{process_lock_event, process_manual_start, process_manual_stop};
use grpc_server::{ParkingAdaptorService, SessionEvent};
use operator::OperatorClient;
use session::Session;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        println!("parking-operator-adaptor v{}", env!("CARGO_PKG_VERSION"));
        print_usage();
        std::process::exit(0);
    }

    // Initialise structured logging.
    tracing_subscriber::fmt::init();

    // Load configuration (08-REQ-7.1 through 08-REQ-7.5).
    let cfg = match config::load_config() {
        Ok(c) => c,
        Err(e) => {
            // 08-REQ-7.E1: exit non-zero on invalid config.
            tracing::error!(error = %e, "failed to load configuration");
            std::process::exit(1);
        }
    };

    // Log version and configuration (08-REQ-8.1).
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        parking_operator_url = %cfg.parking_operator_url,
        data_broker_addr = %cfg.data_broker_addr,
        grpc_port = cfg.grpc_port,
        vehicle_id = %cfg.vehicle_id,
        zone_id = %cfg.zone_id,
        "parking-operator-adaptor starting"
    );

    // Connect to DATA_BROKER with retry (08-REQ-3.1, 08-REQ-3.E3).
    let broker_client = match GrpcBrokerClient::connect(&cfg.data_broker_addr).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to connect to DATA_BROKER after retries");
            std::process::exit(1);
        }
    };

    // Publish initial SessionActive=false (08-REQ-4.3).
    if let Err(e) = broker_client.set_bool(SIGNAL_SESSION_ACTIVE, false).await {
        tracing::error!(error = %e, "failed to publish initial SessionActive=false");
        std::process::exit(1);
    }

    // Subscribe to IsLocked signal (08-REQ-3.2).
    let mut lock_rx = match broker_client.subscribe_bool(SIGNAL_IS_LOCKED).await {
        Ok(rx) => rx,
        Err(e) => {
            tracing::error!(error = %e, "failed to subscribe to IsLocked signal");
            std::process::exit(1);
        }
    };

    // Create operator client.
    let operator_client = OperatorClient::new(&cfg.parking_operator_url);

    // Create event channel for serialized processing (08-REQ-9.1).
    let (event_tx, mut event_rx) = mpsc::channel::<SessionEvent>(64);

    // Build the gRPC service.
    let svc = ParkingAdaptorService::new(event_tx.clone());

    // Bind address (08-REQ-1.1).
    let addr = format!("0.0.0.0:{}", cfg.grpc_port)
        .parse()
        .expect("invalid listen address");

    // Pre-register the shutdown signal handler so that SIGTERM is caught even
    // before the gRPC server's serve_with_shutdown future is polled.
    // tokio::signal::unix::signal() synchronously installs the OS handler on
    // creation — the returned Signal only needs to be polled to *receive* the
    // notification, but the registration itself is immediate. (08-REQ-8.3)
    let shutdown = {
        let ctrl_c = tokio::signal::ctrl_c();
        #[cfg(unix)]
        {
            let mut sigterm = tokio::signal::unix::signal(
                tokio::signal::unix::SignalKind::terminate(),
            )
            .expect("failed to install SIGTERM handler");
            async move {
                tokio::select! {
                    _ = ctrl_c => {
                        tracing::info!("received SIGINT, initiating graceful shutdown");
                    }
                    _ = sigterm.recv() => {
                        tracing::info!("received SIGTERM, initiating graceful shutdown");
                    }
                }
            }
        }
        #[cfg(not(unix))]
        {
            async move {
                ctrl_c.await.ok();
                tracing::info!("received SIGINT, initiating graceful shutdown");
            }
        }
    };

    // Start the gRPC server as a background task.
    let grpc_server = tonic::transport::Server::builder()
        .add_service(ParkingAdaptorServer::new(svc))
        .serve_with_shutdown(addr, shutdown);

    // Log ready message (08-REQ-8.2).
    tracing::info!(
        grpc_addr = %addr,
        "parking-operator-adaptor ready"
    );

    let vehicle_id = cfg.vehicle_id.clone();
    let zone_id = cfg.zone_id.clone();

    // Spawn a task that forwards lock events into the event channel.
    let lock_event_tx = event_tx.clone();
    tokio::spawn(async move {
        loop {
            match lock_rx.recv().await {
                Some(is_locked) => {
                    tracing::info!(is_locked, "received lock event from DATA_BROKER");
                    if lock_event_tx
                        .send(SessionEvent::LockChanged(is_locked))
                        .await
                        .is_err()
                    {
                        tracing::error!("event channel closed, stopping lock event forwarder");
                        break;
                    }
                }
                None => {
                    tracing::error!("lock subscription channel closed");
                    break;
                }
            }
        }
    });

    // Drop the extra sender so event_rx closes when gRPC server and lock forwarder stop.
    drop(event_tx);

    // Run gRPC server and event loop concurrently.
    tokio::select! {
        result = grpc_server => {
            if let Err(e) = result {
                tracing::error!(error = %e, "gRPC server error");
                std::process::exit(1);
            }
            tracing::info!("gRPC server shut down");
        }
        // Event loop: process all events sequentially (08-REQ-9.1, 08-REQ-9.2).
        _ = async {
            let mut session = Session::new();
            while let Some(event) = event_rx.recv().await {
                match event {
                    SessionEvent::LockChanged(is_locked) => {
                        if let Err(e) = process_lock_event(
                            is_locked,
                            &mut session,
                            &operator_client,
                            &broker_client,
                            &vehicle_id,
                            &zone_id,
                        ).await {
                            tracing::error!(error = %e, "failed to process lock event");
                        }
                    }
                    SessionEvent::ManualStart { zone_id: z, reply } => {
                        let result = process_manual_start(
                            &z,
                            &mut session,
                            &operator_client,
                            &broker_client,
                            &vehicle_id,
                        ).await;
                        let _ = reply.send(result);
                    }
                    SessionEvent::ManualStop { reply } => {
                        let result = process_manual_stop(
                            &mut session,
                            &operator_client,
                            &broker_client,
                        ).await;
                        let _ = reply.send(result);
                    }
                    SessionEvent::QueryStatus { reply } => {
                        let state = session.status().cloned();
                        let _ = reply.send(state);
                    }
                    SessionEvent::QueryRate { reply } => {
                        let rate = session.rate().cloned();
                        let _ = reply.send(rate);
                    }
                }
            }
            tracing::info!("event channel closed, event loop exiting");
        } => {}
    }
}

/// Print usage information.
fn print_usage() {
    eprintln!("Usage: parking-operator-adaptor");
    eprintln!();
    eprintln!("Environment:");
    eprintln!("  PARKING_OPERATOR_URL   Operator REST base URL (default: http://localhost:8080)");
    eprintln!("  DATA_BROKER_ADDR       DATA_BROKER gRPC address (default: http://localhost:55556)");
    eprintln!("  GRPC_PORT              gRPC listen port (default: 50053)");
    eprintln!("  VEHICLE_ID             Vehicle identifier (default: DEMO-VIN-001)");
    eprintln!("  ZONE_ID                Default parking zone (default: zone-demo-1)");
    eprintln!();
    eprintln!("The service subscribes to lock/unlock events and manages parking sessions");
    eprintln!("autonomously. Manual override available via gRPC.");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        // Verify the binary compiles and modules are accessible.
        let _ = super::config::Config {
            parking_operator_url: String::new(),
            data_broker_addr: String::new(),
            grpc_port: 0,
            vehicle_id: String::new(),
            zone_id: String::new(),
        };
    }
}
