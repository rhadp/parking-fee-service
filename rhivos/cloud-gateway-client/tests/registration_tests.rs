//! Unit test for registration message format.
//!
//! Tests cover:
//! - TS-04-P1: Registration message format
//!
//! Requirements: [04-REQ-4.1]

use cloud_gateway_client::models::RegistrationMessage;

// ---------------------------------------------------------------------------
// TS-04-P1: Registration message format
// Validates: [04-REQ-4.1]
// ---------------------------------------------------------------------------

#[test]
fn ts_04_p1_registration_message_format() {
    // GIVEN vin = "VIN-001"
    let msg = RegistrationMessage {
        vin: "VIN-001".to_string(),
        status: "online".to_string(),
        timestamp: 1700000000,
    };

    // WHEN RegistrationMessage is serialized
    let json_str = serde_json::to_string(&msg).expect("serialization should succeed");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("should be valid JSON");

    // THEN json contains "vin":"VIN-001"
    assert_eq!(parsed["vin"], "VIN-001");
    // AND json contains "status":"online"
    assert_eq!(parsed["status"], "online");
    // AND json contains "timestamp"
    assert!(
        parsed.get("timestamp").is_some(),
        "registration message should contain a timestamp"
    );
    assert_eq!(parsed["timestamp"], 1700000000);
}
