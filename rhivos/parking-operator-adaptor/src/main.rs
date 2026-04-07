use parking_operator_adaptor::broker::{DataBrokerClient, KuksaBrokerClient};
use parking_operator_adaptor::config::load_config;
use parking_operator_adaptor::event_loop::{
    process_lock_event, process_manual_start, process_manual_stop, SIGNAL_IS_LOCKED,
    SIGNAL_SESSION_ACTIVE,
};
use parking_operator_adaptor::grpc_server::{ParkingAdaptorService, SessionEvent};
use parking_operator_adaptor::operator::OperatorClient;
use parking_operator_adaptor::proto::parking_adaptor::parking_adaptor_service_server::ParkingAdaptorServiceServer;
use parking_operator_adaptor::session::Session;

use std::net::SocketAddr;
use std::process;

use tokio::sync::mpsc;
use tonic::transport::Server;
use tracing::{error, info};

const VERSION: &str = "0.1.0";

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            error!("configuration error: {e}");
            process::exit(1);
        }
    };

    // Log startup info (REQ-8.1)
    info!(
        version = VERSION,
        parking_operator_url = %config.parking_operator_url,
        data_broker_addr = %config.data_broker_addr,
        grpc_port = config.grpc_port,
        vehicle_id = %config.vehicle_id,
        zone_id = %config.zone_id,
        "parking-operator-adaptor starting"
    );

    // Connect to DATA_BROKER with retry (REQ-3.1, REQ-3.E3)
    let mut broker = match KuksaBrokerClient::connect(&config.data_broker_addr).await {
        Ok(b) => b,
        Err(e) => {
            error!("failed to connect to DATA_BROKER: {e}");
            process::exit(1);
        }
    };

    // Publish initial SessionActive=false (REQ-4.3)
    if let Err(e) = broker.set_bool(SIGNAL_SESSION_ACTIVE, false).await {
        error!("failed to publish initial SessionActive=false: {e}");
        // Continue operation per REQ-4.E1
    }

    // Subscribe to lock signal (REQ-3.2)
    let mut lock_stream = match broker.subscribe(SIGNAL_IS_LOCKED).await {
        Ok(s) => s,
        Err(e) => {
            error!("failed to subscribe to lock signal: {e}");
            process::exit(1);
        }
    };

    // Create operator client
    let operator = OperatorClient::new(&config.parking_operator_url);

    // Create event channel for serialized processing (REQ-9.1)
    let (event_tx, mut event_rx) = mpsc::channel::<SessionEvent>(32);

    // Create gRPC service
    let grpc_service = ParkingAdaptorService::new(event_tx.clone());

    // Start gRPC server (REQ-1.1)
    let addr: SocketAddr = ([0, 0, 0, 0], config.grpc_port).into();
    let grpc_server = Server::builder()
        .add_service(ParkingAdaptorServiceServer::new(grpc_service))
        .serve_with_shutdown(addr, async {
            tokio::signal::ctrl_c().await.ok();
        });

    // Clone config values for the event loop task
    let vehicle_id = config.vehicle_id.clone();
    let zone_id = config.zone_id.clone();

    // Spawn DATA_BROKER subscription reader — forwards lock events to the event channel
    let event_tx_sub = event_tx.clone();
    tokio::spawn(async move {
        use tokio_stream::StreamExt;
        while let Some(result) = lock_stream.next().await {
            match result {
                Ok(subscribe_response) => {
                    for datapoint in subscribe_response.entries.values() {
                        if let Some(ref value) = datapoint.value {
                            if let Some(
                                parking_operator_adaptor::proto::kuksa::value::TypedValue::Bool(
                                    is_locked,
                                ),
                            ) = value.typed_value
                            {
                                if event_tx_sub
                                    .send(SessionEvent::LockChanged(is_locked))
                                    .await
                                    .is_err()
                                {
                                    info!("event loop closed, stopping subscription reader");
                                    return;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("lock subscription error: {e}");
                }
            }
        }
        info!("lock subscription stream ended");
    });

    // Spawn the event processing loop (REQ-9.1, REQ-9.2)
    tokio::spawn(async move {
        let mut session = Session::new();

        while let Some(event) = event_rx.recv().await {
            match event {
                SessionEvent::LockChanged(is_locked) => {
                    if let Err(e) = process_lock_event(
                        is_locked, &mut session, &operator, &broker, &vehicle_id, &zone_id,
                    )
                    .await
                    {
                        error!("lock event processing failed: {e}");
                    }
                }
                SessionEvent::ManualStart { zone_id, reply } => {
                    let result = process_manual_start(
                        &zone_id,
                        &mut session,
                        &operator,
                        &broker,
                        &vehicle_id,
                    )
                    .await;
                    let _ = reply.send(result);
                }
                SessionEvent::ManualStop { reply } => {
                    let result =
                        process_manual_stop(&mut session, &operator, &broker).await;
                    let _ = reply.send(result);
                }
                SessionEvent::QueryStatus { reply } => {
                    let _ = reply.send(session.status().cloned());
                }
                SessionEvent::QueryRate { reply } => {
                    let _ = reply.send(session.rate().cloned());
                }
            }
        }
        info!("event loop exiting");
    });

    // Log ready (REQ-8.2)
    info!(addr = %addr, "parking-operator-adaptor ready");

    // Run gRPC server (blocks until shutdown signal) (REQ-8.3)
    if let Err(e) = grpc_server.await {
        error!("gRPC server error: {e}");
        process::exit(1);
    }

    info!("parking-operator-adaptor shutting down");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_compiles() {
        assert!(true);
    }
}
