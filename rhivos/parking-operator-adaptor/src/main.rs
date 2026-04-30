use parking_operator_adaptor::broker::BrokerClient;
use parking_operator_adaptor::config::load_config;
use parking_operator_adaptor::event_loop::{
    run_event_loop, SessionEvent, SIGNAL_IS_LOCKED, SIGNAL_SESSION_ACTIVE,
};
use parking_operator_adaptor::grpc_server::ParkingAdaptorServiceImpl;
use parking_operator_adaptor::operator::OperatorClient;
use parking_operator_adaptor::proto::parking_adaptor::v1::parking_operator_adaptor_service_server::ParkingOperatorAdaptorServiceServer;
use std::net::SocketAddr;
use tokio::sync::mpsc;

const VERSION: &str = "0.1.0";

#[tokio::main]
async fn main() {
    // Initialize structured logging to stderr.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    if let Err(e) = run().await {
        tracing::error!(error = %e, "service exited with error");
        std::process::exit(1);
    }
}

/// Main service entry point.
///
/// Loads configuration, connects to DATA_BROKER, starts the gRPC
/// server, subscribes to lock/unlock events, and runs the event loop
/// until a shutdown signal is received.
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Load and validate configuration (REQ 08-REQ-7.*).
    let config = load_config().map_err(|e| {
        tracing::error!(error = %e, "configuration error");
        e
    })?;

    // Log startup info (REQ 08-REQ-8.1).
    tracing::info!(
        version = VERSION,
        parking_operator_url = %config.parking_operator_url,
        data_broker_addr = %config.data_broker_addr,
        grpc_port = config.grpc_port,
        vehicle_id = %config.vehicle_id,
        zone_id = %config.zone_id,
        "parking-operator-adaptor starting"
    );

    // Connect to DATA_BROKER with retry (REQ 08-REQ-3.1, 08-REQ-3.E3).
    let broker = BrokerClient::connect(&config.data_broker_addr).await?;

    // Publish initial SessionActive=false (REQ 08-REQ-4.3).
    if let Err(e) = broker.publish_bool(SIGNAL_SESSION_ACTIVE, false).await {
        tracing::error!(error = %e, "failed to publish initial SessionActive=false");
    }

    // Subscribe to IsLocked signal (REQ 08-REQ-3.2).
    let lock_rx = broker.subscribe_bool(SIGNAL_IS_LOCKED).await?;

    // Create event channel for serialized processing (REQ 08-REQ-9.1).
    let (event_tx, event_rx) = mpsc::channel::<SessionEvent>(32);

    // Create operator REST client (REQ 08-REQ-2.*).
    let operator = OperatorClient::new(&config.parking_operator_url);

    // Set up gRPC server (REQ 08-REQ-1.1).
    let addr: SocketAddr = format!("0.0.0.0:{}", config.grpc_port).parse()?;
    let grpc_service = ParkingAdaptorServiceImpl::new(event_tx.clone());

    // Shutdown coordination via watch channel.
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Spawn gRPC server with graceful shutdown.
    let mut server_shutdown_rx = shutdown_rx.clone();
    let server_handle = tokio::spawn(async move {
        if let Err(e) = tonic::transport::Server::builder()
            .add_service(ParkingOperatorAdaptorServiceServer::new(grpc_service))
            .serve_with_shutdown(addr, async move {
                let _ = server_shutdown_rx.changed().await;
            })
            .await
        {
            tracing::error!(error = %e, "gRPC server error");
        }
    });

    // Spawn lock event forwarder: reads from DATA_BROKER subscription
    // and forwards to the event channel.
    let lock_event_tx = event_tx.clone();
    let lock_forwarder = tokio::spawn(async move {
        let mut lock_rx = lock_rx;
        while let Some(is_locked) = lock_rx.recv().await {
            tracing::info!(is_locked, "received lock state change from DATA_BROKER");
            if lock_event_tx
                .send(SessionEvent::LockChanged(is_locked))
                .await
                .is_err()
            {
                break;
            }
        }
        tracing::warn!("lock subscription ended");
    });

    // Drop our copy of event_tx so the event loop can detect when all
    // other senders (gRPC server, lock forwarder) are dropped.
    drop(event_tx);

    // Service is ready (REQ 08-REQ-8.2).
    tracing::info!("parking-operator-adaptor ready");

    // Spawn shutdown signal handler. When SIGTERM/SIGINT is received,
    // it shuts down the gRPC server and lock forwarder, which causes
    // the event loop to exit after completing any in-flight operation
    // (REQ 08-REQ-8.3, 08-REQ-8.E1).
    tokio::spawn(async move {
        shutdown_signal().await;
        tracing::info!("shutdown signal received, initiating graceful shutdown");

        // Signal gRPC server to stop accepting new requests.
        let _ = shutdown_tx.send(true);

        // Wait for gRPC server to finish in-flight requests.
        let _ = server_handle.await;

        // Stop the lock forwarder.
        lock_forwarder.abort();
    });

    // Run event loop on the main task. Processes events sequentially
    // until all senders are dropped (channel closed).
    run_event_loop(
        event_rx,
        &operator,
        &broker,
        &config.vehicle_id,
        &config.zone_id,
    )
    .await;

    tracing::info!("parking-operator-adaptor shut down");
    Ok(())
}

/// Wait for a shutdown signal (SIGTERM or SIGINT).
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("received SIGINT");
            }
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM");
            }
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for Ctrl+C");
        tracing::info!("received shutdown signal");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        // Placeholder: verifies the binary crate compiles.
    }
}
