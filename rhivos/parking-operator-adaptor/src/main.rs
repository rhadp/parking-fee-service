//! PARKING_OPERATOR_ADAPTOR binary entry point.
//!
//! Wires together:
//! 1. Configuration from environment variables (08-REQ-7.*)
//! 2. DATA_BROKER connection + subscription (08-REQ-3.*)
//! 3. Initial `Vehicle.Parking.SessionActive = false` publish (08-REQ-4.3)
//! 4. Event loop for sequential session state management (08-REQ-9.*)
//! 5. gRPC server on configured port (08-REQ-1.1)
//! 6. Graceful shutdown on SIGTERM / SIGINT (08-REQ-8.3)

use futures::StreamExt;
use parking_operator_adaptor::{
    broker::BrokerClient,
    config::load_config,
    event_loop::{run_event_loop, BrokerTrait, SessionCommand, SIGNAL_IS_LOCKED, SIGNAL_SESSION_ACTIVE},
    grpc_server::ParkingService,
    operator::OperatorClient,
    proto::{
        kuksa::datapoint::Value as KuksaValue,
        parking::parking_adaptor_server::ParkingAdaptorServer,
    },
    session::Session,
};
use tonic::transport::Server;
use tracing::info;

#[tokio::main]
async fn main() {
    // Initialise structured logging (RUST_LOG controls verbosity).
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // ── Configuration ─────────────────────────────────────────────────────────

    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ERROR: {e}");
            std::process::exit(1);
        }
    };

    // Startup log — version + all config values (08-REQ-8.1).
    info!(
        version = env!("CARGO_PKG_VERSION"),
        parking_operator_url = %config.parking_operator_url,
        data_broker_addr = %config.data_broker_addr,
        grpc_port = config.grpc_port,
        vehicle_id = %config.vehicle_id,
        zone_id = %config.zone_id,
        "parking-operator-adaptor starting"
    );

    // ── DATA_BROKER connection (with retry; exit on failure — 08-REQ-3.E3) ───

    let mut broker = match BrokerClient::connect(&config.data_broker_addr).await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("ERROR: {}", e.0);
            std::process::exit(1);
        }
    };

    // Publish initial SessionActive=false (08-REQ-4.3).
    if let Err(e) = broker.set_bool(SIGNAL_SESSION_ACTIVE, false).await {
        tracing::warn!("failed to publish initial SessionActive=false: {}", e.0);
    }

    // Subscribe to IsLocked signal (08-REQ-3.2).
    let mut lock_stream = match broker.subscribe_bool(SIGNAL_IS_LOCKED).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("ERROR: failed to subscribe to IsLocked: {}", e.0);
            std::process::exit(1);
        }
    };

    // ── Operator client ───────────────────────────────────────────────────────

    let operator = OperatorClient::new(&config.parking_operator_url);

    // ── Event loop ────────────────────────────────────────────────────────────

    let (tx, rx) = tokio::sync::mpsc::channel::<SessionCommand>(128);
    let vehicle_id = config.vehicle_id.clone();
    let zone_id_loop = config.zone_id.clone();

    tokio::spawn(run_event_loop(
        rx,
        Session::new(),
        operator,
        broker,
        vehicle_id,
        zone_id_loop,
    ));

    // ── Lock subscription forwarder ───────────────────────────────────────────
    // Reads the DATA_BROKER IsLocked stream and forwards bool values to the
    // event loop as SessionCommand::LockChanged.

    let tx_lock = tx.clone();
    tokio::spawn(async move {
        while let Some(item) = lock_stream.next().await {
            match item {
                Ok(subscribe_resp) => {
                    for update in subscribe_resp.updates {
                        if let Some(entry) = update.entry {
                            if let Some(dp) = entry.value {
                                if let Some(KuksaValue::BoolValue(v)) = dp.value {
                                    let _ = tx_lock
                                        .send(SessionCommand::LockChanged(v))
                                        .await;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("DATA_BROKER subscription stream error: {e}");
                    break;
                }
            }
        }
        tracing::warn!("DATA_BROKER IsLocked subscription stream ended");
    });

    // ── gRPC server ───────────────────────────────────────────────────────────

    let addr = format!("0.0.0.0:{}", config.grpc_port)
        .parse()
        .expect("invalid gRPC bind address");

    let service = ParkingService::new(tx);

    // Ready log (08-REQ-8.2).
    info!(
        port = config.grpc_port,
        "parking-operator-adaptor ready"
    );

    // ── Graceful shutdown (08-REQ-8.3) ────────────────────────────────────────

    let shutdown_signal = async {
        #[cfg(unix)]
        {
            let mut sigterm = tokio::signal::unix::signal(
                tokio::signal::unix::SignalKind::terminate(),
            )
            .expect("failed to register SIGTERM handler");

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("received SIGINT — initiating graceful shutdown");
                }
                _ = sigterm.recv() => {
                    info!("received SIGTERM — initiating graceful shutdown");
                }
            }
        }

        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to listen for Ctrl+C");
            info!("received Ctrl+C — initiating graceful shutdown");
        }
    };

    Server::builder()
        .add_service(ParkingAdaptorServer::new(service))
        .serve_with_shutdown(addr, shutdown_signal)
        .await
        .expect("gRPC server error");

    info!("parking-operator-adaptor stopped");
}

#[cfg(test)]
mod tests {
    /// Verifies the crate compiles successfully (01-REQ-8.1, TS-01-26).
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
