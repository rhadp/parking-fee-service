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

use broker::DataBrokerClient;
use event_loop::SessionEvent;

#[tokio::main]
async fn main() {
    // Initialize structured logging (08-REQ-8.1)
    tracing_subscriber::fmt::init();

    // Load configuration from environment (08-REQ-7.*)
    let config = match config::load_config() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "configuration error");
            std::process::exit(1);
        }
    };

    // Log startup info with all configuration values (08-REQ-8.1)
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        parking_operator_url = %config.parking_operator_url,
        data_broker_addr = %config.data_broker_addr,
        grpc_port = config.grpc_port,
        vehicle_id = %config.vehicle_id,
        zone_id = %config.zone_id,
        "parking-operator-adaptor starting"
    );

    // Connect to DATA_BROKER with retry (08-REQ-3.1, 08-REQ-3.E3)
    let broker_client = match broker::GrpcBrokerClient::connect(&config.data_broker_addr).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to connect to DATA_BROKER");
            std::process::exit(1);
        }
    };

    // Publish initial SessionActive=false (08-REQ-4.3)
    if let Err(e) = broker_client
        .set_bool(broker::SIGNAL_SESSION_ACTIVE, false)
        .await
    {
        tracing::error!(error = %e, "failed to publish initial SessionActive");
    }

    // Subscribe to IsLocked signal (08-REQ-3.2)
    let lock_rx = match broker_client
        .subscribe_bool(broker::SIGNAL_IS_LOCKED)
        .await
    {
        Ok(rx) => rx,
        Err(e) => {
            tracing::error!(error = %e, "failed to subscribe to IsLocked");
            std::process::exit(1);
        }
    };

    // Create operator REST client (08-REQ-2.*)
    let operator_client = operator::OperatorClient::new(&config.parking_operator_url);

    // Create event channel for sequential processing (08-REQ-9.1)
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<SessionEvent>(32);

    // Spawn lock subscription forwarder (08-REQ-3.3, 08-REQ-3.4)
    let lock_event_tx = event_tx.clone();
    spawn_lock_forwarder(lock_rx, lock_event_tx);

    // Set up and spawn gRPC server (08-REQ-1.1)
    let addr: std::net::SocketAddr = format!("0.0.0.0:{}", config.grpc_port)
        .parse()
        .expect("invalid gRPC address");

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let service = grpc_server::ParkingAdaptorService::new(event_tx);

    tokio::spawn(async move {
        if let Err(e) = tonic::transport::Server::builder()
            .add_service(grpc_server::ParkingOperatorAdaptorServiceServer::new(
                service,
            ))
            .serve_with_shutdown(addr, async {
                let _ = shutdown_rx.await;
            })
            .await
        {
            tracing::error!(error = %e, "gRPC server error");
        }
    });

    // Register signal handlers (08-REQ-8.3)
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("failed to register SIGTERM handler");

    // Log ready message (08-REQ-8.2)
    tracing::info!("ready");

    // Event processing loop.
    //
    // Processes events sequentially: one event at a time. When an event
    // handler is running (e.g. during an in-flight REST call), signal
    // handlers are not polled — SIGTERM during an in-flight call will
    // be detected on the next iteration, after the call completes
    // (08-REQ-8.E1).
    let mut session = session::Session::new();

    loop {
        tokio::select! {
            biased;

            event = event_rx.recv() => {
                match event {
                    Some(evt) => {
                        event_loop::process_event(
                            evt,
                            &mut session,
                            &operator_client,
                            &broker_client,
                            &config.vehicle_id,
                            &config.zone_id,
                        )
                        .await;
                    }
                    None => {
                        tracing::warn!("event channel closed");
                        break;
                    }
                }
            }

            _ = tokio::signal::ctrl_c() => {
                tracing::info!("received SIGINT, shutting down");
                break;
            }

            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM, shutting down");
                break;
            }
        }
    }

    // Graceful shutdown: signal the gRPC server to stop (08-REQ-8.3)
    let _ = shutdown_tx.send(());
    tracing::info!("parking-operator-adaptor stopped");
}

/// Spawn a background task that forwards lock state changes from the
/// DATA_BROKER subscription to the event channel.
fn spawn_lock_forwarder(
    mut lock_rx: tokio::sync::mpsc::Receiver<bool>,
    event_tx: tokio::sync::mpsc::Sender<SessionEvent>,
) {
    tokio::spawn(async move {
        while let Some(is_locked) = lock_rx.recv().await {
            tracing::info!(is_locked, "received lock event from DATA_BROKER");
            if event_tx
                .send(SessionEvent::LockChanged(is_locked))
                .await
                .is_err()
            {
                tracing::warn!("event channel closed, stopping lock forwarder");
                break;
            }
        }
        tracing::info!("lock subscription forwarder stopped");
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        // Verify the binary crate compiles successfully.
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
    }
}
