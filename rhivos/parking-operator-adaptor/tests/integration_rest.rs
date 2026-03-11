//! Integration test: REST start/stop cycle (TS-08-9, TS-08-10)
//!
//! Directly calls the mock PARKING_OPERATOR REST endpoints to validate the API contract.
//!
//! Requires infrastructure running:
//!   - Mock PARKING_OPERATOR on port 8080
//!
//! Run with: `cargo test -p parking-operator-adaptor --features integration --test integration_rest`

#![cfg(feature = "integration")]

use std::time::Duration;

const OPERATOR_URL: &str = "http://localhost:8080";

/// Helper to check if the mock operator is reachable.
async fn operator_available() -> bool {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap()
        .get(format!("{OPERATOR_URL}/health"))
        .send()
        .await
        .is_ok()
}

/// TS-08-9: POST /parking/start returns session_id and status.
/// POST /parking/stop returns session_id, duration, fee, and status.
#[tokio::test]
async fn test_rest_start_stop_cycle() {
    if !operator_available().await {
        eprintln!("SKIP: mock operator not running on port 8080");
        return;
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    // POST /parking/start
    let start_resp = client
        .post(format!("{OPERATOR_URL}/parking/start"))
        .json(&serde_json::json!({
            "vehicle_id": "VIN-001",
            "zone_id": "zone-1",
            "timestamp": chrono::Utc::now().timestamp()
        }))
        .send()
        .await
        .expect("POST /parking/start should succeed");

    assert_eq!(
        start_resp.status().as_u16(),
        200,
        "POST /parking/start should return HTTP 200"
    );

    let start_body: serde_json::Value = start_resp.json().await.unwrap();
    let session_id = start_body["session_id"]
        .as_str()
        .expect("response should contain session_id string");
    assert!(
        !session_id.is_empty(),
        "session_id should be a non-empty string (UUID)"
    );

    let status = start_body["status"]
        .as_str()
        .expect("response should contain status string");
    assert_eq!(status, "active", "status should be 'active'");

    // Wait to accumulate some duration
    tokio::time::sleep(Duration::from_secs(2)).await;

    // POST /parking/stop
    let stop_resp = client
        .post(format!("{OPERATOR_URL}/parking/stop"))
        .json(&serde_json::json!({
            "session_id": session_id,
            "timestamp": chrono::Utc::now().timestamp()
        }))
        .send()
        .await
        .expect("POST /parking/stop should succeed");

    assert_eq!(
        stop_resp.status().as_u16(),
        200,
        "POST /parking/stop should return HTTP 200"
    );

    let stop_body: serde_json::Value = stop_resp.json().await.unwrap();
    assert_eq!(
        stop_body["session_id"].as_str().unwrap(),
        session_id,
        "stop response session_id should match start session_id"
    );
    assert!(
        stop_body["duration"].as_i64().unwrap() >= 0,
        "duration should be non-negative"
    );
    assert!(
        stop_body["fee"].as_f64().unwrap() >= 0.0,
        "fee should be non-negative"
    );
    assert_eq!(
        stop_body["status"].as_str().unwrap(),
        "completed",
        "status should be 'completed'"
    );
}

/// TS-08-10: GET /parking/status/{session_id} returns session status with rate info.
#[tokio::test]
async fn test_rest_status_query() {
    if !operator_available().await {
        eprintln!("SKIP: mock operator not running on port 8080");
        return;
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    // Start a session first
    let start_resp = client
        .post(format!("{OPERATOR_URL}/parking/start"))
        .json(&serde_json::json!({
            "vehicle_id": "VIN-001",
            "zone_id": "zone-1",
            "timestamp": chrono::Utc::now().timestamp()
        }))
        .send()
        .await
        .expect("POST /parking/start should succeed");

    let start_body: serde_json::Value = start_resp.json().await.unwrap();
    let session_id = start_body["session_id"].as_str().unwrap();

    // GET /parking/status/{session_id}
    let status_resp = client
        .get(format!("{OPERATOR_URL}/parking/status/{session_id}"))
        .send()
        .await
        .expect("GET /parking/status should succeed");

    assert_eq!(
        status_resp.status().as_u16(),
        200,
        "GET /parking/status should return HTTP 200"
    );

    let status_body: serde_json::Value = status_resp.json().await.unwrap();
    assert_eq!(
        status_body["session_id"].as_str().unwrap(),
        session_id,
        "status response session_id should match"
    );
    assert_eq!(
        status_body["status"].as_str().unwrap(),
        "active",
        "status should be 'active'"
    );
    assert!(
        status_body.get("rate_type").is_some(),
        "response should contain rate_type"
    );
    let rate_type = status_body["rate_type"].as_str().unwrap();
    assert!(
        rate_type == "per_hour" || rate_type == "flat_fee",
        "rate_type should be 'per_hour' or 'flat_fee'"
    );
    assert!(
        status_body["rate_amount"].as_f64().is_some(),
        "response should contain rate_amount"
    );
    assert!(
        status_body["currency"].as_str().is_some(),
        "response should contain currency"
    );

    // Clean up: stop the session
    let _ = client
        .post(format!("{OPERATOR_URL}/parking/stop"))
        .json(&serde_json::json!({
            "session_id": session_id,
            "timestamp": chrono::Utc::now().timestamp()
        }))
        .send()
        .await;
}
