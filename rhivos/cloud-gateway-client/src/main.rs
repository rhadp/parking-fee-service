mod broker_client;
mod command_validator;
mod config;
mod errors;
mod models;
mod nats_client;
mod telemetry;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(first) = args.first() {
        if first == "serve" {
            // Service entry point — implemented in task group 7.
            todo!("implement service startup in task group 7");
        }
        if first.starts_with('-') {
            eprintln!("Usage: cloud-gateway-client [serve]");
            std::process::exit(1);
        }
    }
    println!("cloud-gateway-client v0.1.0");
}

#[cfg(test)]
mod tests {
    use crate::models::RegistrationMessage;

    /// Verifies the crate compiles successfully (01-REQ-8.1, TS-01-26).
    #[test]
    fn it_compiles() {
        assert!(true);
    }

    // ------------------------------------------------------------------
    // TS-04-P1: Registration message serializes with required fields
    // ------------------------------------------------------------------

    /// TS-04-P1: `RegistrationMessage` serializes to JSON with `vin`, `status`,
    /// and `timestamp` fields.
    ///
    /// Validates [04-REQ-4.1]
    #[test]
    fn test_registration_message_format() {
        let msg = RegistrationMessage {
            vin: "VIN-001".to_string(),
            status: "online".to_string(),
            timestamp: 1_700_000_000,
        };
        let json = serde_json::to_string(&msg).expect("RegistrationMessage must serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("serialized output must be valid JSON");

        assert_eq!(parsed["vin"], "VIN-001");
        assert_eq!(parsed["status"], "online");
        assert!(
            parsed["timestamp"].is_number(),
            "timestamp must be a number"
        );
    }

    // ------------------------------------------------------------------
    // Integration property tests (require external infrastructure)
    // TS-04-P3, TS-04-P4, TS-04-P6
    // ------------------------------------------------------------------

    /// TS-04-P3: Command passthrough fidelity.
    ///
    /// The original NATS payload bytes must be written to DATA_BROKER
    /// (`Vehicle.Command.Door.Lock`) without any modification or re-serialization.
    ///
    /// Validates [04-REQ-6.3], [04-REQ-6.4]
    #[test]
    #[ignore = "integration test — requires running NATS and DATA_BROKER containers"]
    fn test_property_command_passthrough_fidelity() {
        // Full end-to-end verification implemented in task group 8 (TS-04-10).
        todo!("implement in task group 8")
    }

    /// TS-04-P4: Response relay fidelity.
    ///
    /// DATA_BROKER `Vehicle.Command.Door.Response` JSON must be relayed verbatim
    /// to `vehicles.{VIN}.command_responses` on NATS without modification.
    ///
    /// Validates [04-REQ-7.1], [04-REQ-7.2]
    #[test]
    #[ignore = "integration test — requires running NATS and DATA_BROKER containers"]
    fn test_property_response_relay_fidelity() {
        // Full end-to-end verification implemented in task group 8 (TS-04-11).
        todo!("implement in task group 8")
    }

    /// TS-04-P6: Startup determinism.
    ///
    /// A failure at any startup step (config → NATS → DATA_BROKER → registration
    /// → processing) must prevent all subsequent steps from executing, and the
    /// service must exit with code 1.
    ///
    /// Validates [04-REQ-9.1], [04-REQ-9.2]
    #[test]
    #[ignore = "integration test — requires full service binary with injectable failures"]
    fn test_property_startup_determinism() {
        // Full end-to-end verification implemented in task group 8.
        todo!("implement in task group 8")
    }
}
