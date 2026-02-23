//! CLOUD_GATEWAY_CLIENT entry point.
//!
//! Connects to the MQTT broker and DATA_BROKER via gRPC over Unix Domain
//! Sockets. The service bridges commands from the cloud to DATA_BROKER
//! and publishes telemetry and command responses back to MQTT.
//!
//! Configuration is via environment variables:
//! - `DATABROKER_UDS_PATH`: UDS socket path for DATA_BROKER (default: `/tmp/kuksa-databroker.sock`)
//! - `MQTT_BROKER_ADDR`: MQTT broker host:port (default: `localhost:1883`)
//! - `VEHICLE_VIN`: Vehicle identification number (default: `VIN12345`)

use cloud_gateway_client::service::ServiceConfig;
use tracing_subscriber::EnvFilter;

/// Default UDS socket path for DATA_BROKER.
const DEFAULT_UDS_PATH: &str = "/tmp/kuksa-databroker.sock";

/// Default MQTT broker address.
const DEFAULT_MQTT_ADDR: &str = "localhost:1883";

/// Default vehicle identification number.
const DEFAULT_VIN: &str = "VIN12345";

#[tokio::main]
async fn main() {
    // Initialize tracing with env filter (RUST_LOG)
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = resolve_config();

    tracing::info!(
        databroker = %config.databroker_endpoint,
        mqtt = format!("{}:{}", config.mqtt_host, config.mqtt_port),
        vin = %config.vin,
        "starting cloud-gateway-client"
    );

    if let Err(e) = cloud_gateway_client::service::run(config).await {
        tracing::error!(error = %e, "cloud-gateway-client exited with error");
        std::process::exit(1);
    }
}

/// Resolve service configuration from environment variables.
fn resolve_config() -> ServiceConfig {
    // DATA_BROKER endpoint
    let databroker_endpoint = resolve_databroker_endpoint();

    // MQTT broker address
    let mqtt_addr = std::env::var("MQTT_BROKER_ADDR").unwrap_or_else(|_| DEFAULT_MQTT_ADDR.to_string());
    let (mqtt_host, mqtt_port) = parse_mqtt_addr(&mqtt_addr);

    // Vehicle VIN
    let vin = std::env::var("VEHICLE_VIN").unwrap_or_else(|_| DEFAULT_VIN.to_string());

    ServiceConfig {
        databroker_endpoint,
        mqtt_host,
        mqtt_port,
        vin,
    }
}

/// Resolve the DATA_BROKER endpoint from environment variables.
///
/// Priority:
/// 1. `DATABROKER_UDS_PATH` env var (formatted as `unix://` URI)
/// 2. Default UDS path: `/tmp/kuksa-databroker.sock`
fn resolve_databroker_endpoint() -> String {
    if let Ok(uds_path) = std::env::var("DATABROKER_UDS_PATH") {
        if uds_path.starts_with("unix://") {
            return uds_path;
        }
        return format!("unix://{uds_path}");
    }

    format!("unix://{DEFAULT_UDS_PATH}")
}

/// Parse an MQTT broker address string into host and port.
fn parse_mqtt_addr(addr: &str) -> (String, u16) {
    if let Some((host, port_str)) = addr.rsplit_once(':') {
        if let Ok(port) = port_str.parse::<u16>() {
            return (host.to_string(), port);
        }
    }
    (addr.to_string(), 1883)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mqtt_addr_with_port() {
        let (host, port) = parse_mqtt_addr("localhost:1883");
        assert_eq!(host, "localhost");
        assert_eq!(port, 1883);
    }

    #[test]
    fn test_parse_mqtt_addr_custom_port() {
        let (host, port) = parse_mqtt_addr("mqtt.example.com:8883");
        assert_eq!(host, "mqtt.example.com");
        assert_eq!(port, 8883);
    }

    #[test]
    fn test_parse_mqtt_addr_no_port() {
        let (host, port) = parse_mqtt_addr("localhost");
        assert_eq!(host, "localhost");
        assert_eq!(port, 1883);
    }

    #[test]
    fn test_parse_mqtt_addr_invalid_port() {
        let (host, port) = parse_mqtt_addr("localhost:invalid");
        assert_eq!(host, "localhost:invalid");
        assert_eq!(port, 1883);
    }
}
