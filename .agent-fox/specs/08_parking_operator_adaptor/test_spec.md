# Test Specification: PARKING_OPERATOR_ADAPTOR

## Overview

Tests cover gRPC service interface (unit), PARKING_OPERATOR REST client with retry logic (unit), DATA_BROKER lock event subscription and autonomous session management (unit + integration), session state management (unit), override mechanism (unit + integration), configuration (unit), and end-to-end flows (integration). Unit tests use mock HTTP servers and mock DATA_BROKER clients. Integration tests require a running Kuksa Databroker container and a mock PARKING_OPERATOR HTTP server. Property tests use the `proptest` crate for Rust.

## Test Cases

### TS-08-1: gRPC Server Starts on Configured Port

**Requirement:** 08-REQ-1.1
**Type:** unit
**Description:** Verify the gRPC server listens on the port from GRPC_PORT env var.

**Preconditions:**
- None.

**Input:**
- Case 1: GRPC_PORT not set (default 50053).
- Case 2: GRPC_PORT=50099.

**Expected:**
- Case 1: Server binds to port 50053.
- Case 2: Server binds to port 50099.

**Assertion pseudocode:**
```
// Case 1
unset_env("GRPC_PORT")
config = load_config()
ASSERT config.grpc_port == 50053

// Case 2
set_env("GRPC_PORT", "50099")
config = load_config()
ASSERT config.grpc_port == 50099
```

### TS-08-2: StartSession RPC Returns Session Info

**Requirement:** 08-REQ-1.2
**Type:** unit
**Description:** Verify StartSession calls the operator and returns session_id, status, and rate.

**Preconditions:**
- Mock PARKING_OPERATOR returns `{session_id: "sess-1", status: "active", rate: {type: "per_hour", amount: 2.50, currency: "EUR"}}`.
- No active session.

**Input:**
- gRPC StartSession(zone_id="zone-a").

**Expected:**
- Response contains session_id="sess-1", rate with type="per_hour", amount=2.50, currency="EUR".
- Session state is active.

**Assertion pseudocode:**
```
mock_operator.on_start_return(start_response)
resp = grpc_client.start_session("zone-a")
ASSERT resp.session_id == "sess-1"
ASSERT resp.rate.rate_type == "per_hour"
ASSERT resp.rate.amount == 2.50
ASSERT resp.rate.currency == "EUR"
ASSERT session.is_active() == true
```

### TS-08-3: StopSession RPC Returns Stop Info

**Requirement:** 08-REQ-1.3
**Type:** unit
**Description:** Verify StopSession calls the operator and returns duration and cost.

**Preconditions:**
- Active session with session_id="sess-1".
- Mock PARKING_OPERATOR returns `{session_id: "sess-1", status: "completed", duration_seconds: 3600, total_amount: 2.50, currency: "EUR"}`.

**Input:**
- gRPC StopSession().

**Expected:**
- Response contains session_id="sess-1", duration_seconds=3600, total_amount=2.50, currency="EUR".
- Session state is inactive.

**Assertion pseudocode:**
```
session.start("sess-1", "zone-a", now(), rate)
mock_operator.on_stop_return(stop_response)
resp = grpc_client.stop_session()
ASSERT resp.session_id == "sess-1"
ASSERT resp.duration_seconds == 3600
ASSERT resp.total_amount == 2.50
ASSERT session.is_active() == false
```

### TS-08-4: GetStatus Returns Active Session

**Requirement:** 08-REQ-1.4
**Type:** unit
**Description:** Verify GetStatus returns session details when a session is active.

**Preconditions:**
- Active session: session_id="sess-1", zone_id="zone-a", start_time=1700000000, rate=(per_hour, 2.50, EUR).

**Input:**
- gRPC GetStatus().

**Expected:**
- Response: active=true, session_id="sess-1", zone_id="zone-a", start_time=1700000000, rate present.

**Assertion pseudocode:**
```
session.start("sess-1", "zone-a", 1700000000, rate)
resp = grpc_client.get_status()
ASSERT resp.active == true
ASSERT resp.session_id == "sess-1"
ASSERT resp.zone_id == "zone-a"
ASSERT resp.start_time == 1700000000
ASSERT resp.rate.rate_type == "per_hour"
```

### TS-08-5: GetStatus Returns Inactive When No Session

**Requirement:** 08-REQ-1.4
**Type:** unit
**Description:** Verify GetStatus returns active=false when no session exists.

**Preconditions:**
- No active session.

**Input:**
- gRPC GetStatus().

**Expected:**
- Response: active=false, empty session_id, empty zone_id.

**Assertion pseudocode:**
```
resp = grpc_client.get_status()
ASSERT resp.active == false
ASSERT resp.session_id == ""
```

### TS-08-6: GetRate Returns Cached Rate

**Requirement:** 08-REQ-1.5
**Type:** unit
**Description:** Verify GetRate returns the rate from the active session.

**Preconditions:**
- Active session with rate=(flat_fee, 5.00, EUR).

**Input:**
- gRPC GetRate().

**Expected:**
- Response: rate_type="flat_fee", amount=5.00, currency="EUR".

**Assertion pseudocode:**
```
session.start("sess-1", "zone-a", now(), Rate { rate_type: "flat_fee", amount: 5.00, currency: "EUR" })
resp = grpc_client.get_rate()
ASSERT resp.rate_type == "flat_fee"
ASSERT resp.amount == 5.00
ASSERT resp.currency == "EUR"
```

### TS-08-7: GetRate Returns Empty When No Session

**Requirement:** 08-REQ-1.5
**Type:** unit
**Description:** Verify GetRate returns empty when no session is active.

**Preconditions:**
- No active session.

**Input:**
- gRPC GetRate().

**Expected:**
- Response: empty rate (rate_type="", amount=0, currency="").

**Assertion pseudocode:**
```
resp = grpc_client.get_rate()
ASSERT resp.rate_type == ""
ASSERT resp.amount == 0.0
```

### TS-08-8: Operator Start Session REST Call

**Requirement:** 08-REQ-2.1
**Type:** unit
**Description:** Verify the adaptor sends correct POST /parking/start to the operator.

**Preconditions:**
- Mock HTTP server listening.

**Input:**
- operator.start_session("DEMO-VIN-001", "zone-a").

**Expected:**
- POST request to /parking/start with body containing vehicle_id, zone_id, timestamp.
- Response parsed into StartResponse.

**Assertion pseudocode:**
```
mock_server = start_mock_http()
mock_server.expect_post("/parking/start")
    .with_body_containing("vehicle_id", "zone_id", "timestamp")
    .respond_with(json!({session_id: "s1", status: "active", rate: {type: "per_hour", amount: 2.5, currency: "EUR"}}))
client = OperatorClient::new(mock_server.url())
resp = client.start_session("DEMO-VIN-001", "zone-a").await
ASSERT resp.session_id == "s1"
ASSERT resp.rate.rate_type == "per_hour"
ASSERT mock_server.received_post("/parking/start")
```

### TS-08-9: Operator Stop Session REST Call

**Requirement:** 08-REQ-2.2
**Type:** unit
**Description:** Verify the adaptor sends correct POST /parking/stop to the operator.

**Preconditions:**
- Mock HTTP server listening.

**Input:**
- operator.stop_session("sess-1").

**Expected:**
- POST request to /parking/stop with body containing session_id, timestamp.
- Response parsed into StopResponse.

**Assertion pseudocode:**
```
mock_server.expect_post("/parking/stop")
    .with_body_containing("session_id", "timestamp")
    .respond_with(json!({session_id: "sess-1", status: "completed", duration_seconds: 3600, total_amount: 2.50, currency: "EUR"}))
resp = client.stop_session("sess-1").await
ASSERT resp.session_id == "sess-1"
ASSERT resp.duration_seconds == 3600
ASSERT resp.total_amount == 2.50
```

### TS-08-10: Operator Start Response Parsing

**Requirement:** 08-REQ-2.3
**Type:** unit
**Description:** Verify the start response is parsed and stored in session state.

**Preconditions:**
- Mock operator returns valid start response.

**Input:**
- Process a lock event (or manual StartSession).

**Expected:**
- Session state populated: session_id from response, rate cached.

**Assertion pseudocode:**
```
mock_operator.on_start_return({session_id: "s1", status: "active", rate: {type: "per_hour", amount: 2.5, currency: "EUR"}})
process_lock_event(true)
ASSERT session.status().session_id == "s1"
ASSERT session.rate().rate_type == "per_hour"
ASSERT session.rate().amount == 2.5
```

### TS-08-11: Lock Event Starts Session

**Requirement:** 08-REQ-3.3
**Type:** unit
**Description:** Verify a lock event (IsLocked=true) triggers session start.

**Preconditions:**
- No active session. Mock operator returns success.

**Input:**
- Lock event: IsLocked changes to true.

**Expected:**
- Operator start_session called with VEHICLE_ID and ZONE_ID.
- Session becomes active.
- Vehicle.Parking.SessionActive set to true.

**Assertion pseudocode:**
```
mock_operator.on_start_return(start_response)
process_lock_event(is_locked=true)
ASSERT mock_operator.start_called_with("DEMO-VIN-001", "zone-demo-1")
ASSERT session.is_active() == true
ASSERT mock_broker.last_set_bool == ("Vehicle.Parking.SessionActive", true)
```

### TS-08-12: Unlock Event Stops Session

**Requirement:** 08-REQ-3.4
**Type:** unit
**Description:** Verify an unlock event (IsLocked=false) triggers session stop.

**Preconditions:**
- Active session with session_id="sess-1". Mock operator returns success.

**Input:**
- Unlock event: IsLocked changes to false.

**Expected:**
- Operator stop_session called with "sess-1".
- Session becomes inactive.
- Vehicle.Parking.SessionActive set to false.

**Assertion pseudocode:**
```
session.start("sess-1", "zone-a", now(), rate)
mock_operator.on_stop_return(stop_response)
process_lock_event(is_locked=false)
ASSERT mock_operator.stop_called_with("sess-1")
ASSERT session.is_active() == false
ASSERT mock_broker.last_set_bool == ("Vehicle.Parking.SessionActive", false)
```

### TS-08-13: SessionActive Set True on Start

**Requirement:** 08-REQ-4.1
**Type:** unit
**Description:** Verify Vehicle.Parking.SessionActive is set to true when session starts.

**Preconditions:**
- Mock broker available.

**Input:**
- Successful session start (via lock event or manual).

**Expected:**
- broker.set_bool("Vehicle.Parking.SessionActive", true) called.

**Assertion pseudocode:**
```
process_start_session()
ASSERT mock_broker.set_bool_calls contains ("Vehicle.Parking.SessionActive", true)
```

### TS-08-14: SessionActive Set False on Stop

**Requirement:** 08-REQ-4.2
**Type:** unit
**Description:** Verify Vehicle.Parking.SessionActive is set to false when session stops.

**Preconditions:**
- Active session. Mock broker available.

**Input:**
- Successful session stop.

**Expected:**
- broker.set_bool("Vehicle.Parking.SessionActive", false) called.

**Assertion pseudocode:**
```
process_stop_session()
ASSERT mock_broker.set_bool_calls contains ("Vehicle.Parking.SessionActive", false)
```

### TS-08-15: Initial SessionActive Published

**Requirement:** 08-REQ-4.3
**Type:** integration
**Description:** Verify the service publishes SessionActive=false on startup.

**Preconditions:**
- DATA_BROKER container running.

**Input:**
- Start parking-operator-adaptor.

**Expected:**
- Vehicle.Parking.SessionActive is false in DATA_BROKER.

**Assertion pseudocode:**
```
start_adaptor()
wait_for_ready()
value = get_signal("Vehicle.Parking.SessionActive")
ASSERT value == false
```

### TS-08-16: Manual StartSession Override

**Requirement:** 08-REQ-5.1
**Type:** unit
**Description:** Verify manual StartSession works regardless of lock state.

**Preconditions:**
- No active session. Lock state is false (unlocked).

**Input:**
- gRPC StartSession(zone_id="zone-manual").

**Expected:**
- Session starts with operator using zone-manual.
- Session becomes active.

**Assertion pseudocode:**
```
mock_broker.lock_state = false
mock_operator.on_start_return(start_response)
resp = grpc_client.start_session("zone-manual")
ASSERT resp.session_id != ""
ASSERT session.is_active() == true
```

### TS-08-17: Manual StopSession Override

**Requirement:** 08-REQ-5.2
**Type:** unit
**Description:** Verify manual StopSession works regardless of lock state.

**Preconditions:**
- Active session. Lock state is true (locked).

**Input:**
- gRPC StopSession().

**Expected:**
- Session stops with operator.
- Session becomes inactive.

**Assertion pseudocode:**
```
session.start("sess-1", "zone-a", now(), rate)
mock_broker.lock_state = true
mock_operator.on_stop_return(stop_response)
resp = grpc_client.stop_session()
ASSERT resp.session_id == "sess-1"
ASSERT session.is_active() == false
```

### TS-08-18: Configuration Defaults

**Requirement:** 08-REQ-7.1, 08-REQ-7.2, 08-REQ-7.3, 08-REQ-7.4, 08-REQ-7.5
**Type:** unit
**Description:** Verify all config env vars have correct defaults.

**Preconditions:**
- No env vars set.

**Input:**
- load_config().

**Expected:**
- parking_operator_url="http://localhost:8080", data_broker_addr="http://localhost:55556", grpc_port=50053, vehicle_id="DEMO-VIN-001", zone_id="zone-demo-1".

**Assertion pseudocode:**
```
clear_all_env()
config = load_config()
ASSERT config.parking_operator_url == "http://localhost:8080"
ASSERT config.data_broker_addr == "http://localhost:55556"
ASSERT config.grpc_port == 50053
ASSERT config.vehicle_id == "DEMO-VIN-001"
ASSERT config.zone_id == "zone-demo-1"
```

### TS-08-19: Configuration Custom Values

**Requirement:** 08-REQ-7.1, 08-REQ-7.2, 08-REQ-7.3, 08-REQ-7.4, 08-REQ-7.5
**Type:** unit
**Description:** Verify all config env vars are read from the environment.

**Preconditions:**
- Env vars set to custom values.

**Input:**
- load_config() with custom env vars.

**Expected:**
- Config reflects custom values.

**Assertion pseudocode:**
```
set_env("PARKING_OPERATOR_URL", "http://op.example.com:9090")
set_env("DATA_BROKER_ADDR", "http://10.0.0.5:55556")
set_env("GRPC_PORT", "50099")
set_env("VEHICLE_ID", "VIN-CUSTOM-123")
set_env("ZONE_ID", "zone-custom-1")
config = load_config()
ASSERT config.parking_operator_url == "http://op.example.com:9090"
ASSERT config.data_broker_addr == "http://10.0.0.5:55556"
ASSERT config.grpc_port == 50099
ASSERT config.vehicle_id == "VIN-CUSTOM-123"
ASSERT config.zone_id == "zone-custom-1"
```

### TS-08-20: Startup Logging

**Requirement:** 08-REQ-8.1, 08-REQ-8.2
**Type:** integration
**Description:** Verify the service logs config and ready message on startup.

**Preconditions:**
- DATA_BROKER container running. Mock PARKING_OPERATOR running.

**Input:**
- Start parking-operator-adaptor.

**Expected:**
- Log contains version, PARKING_OPERATOR_URL, DATA_BROKER_ADDR, GRPC_PORT, VEHICLE_ID, ZONE_ID.
- Log contains "ready" message.

**Assertion pseudocode:**
```
output = start_and_capture_logs()
ASSERT output contains "parking-operator-adaptor"
ASSERT output contains "localhost:8080" OR PARKING_OPERATOR_URL value
ASSERT output contains "localhost:55556" OR DATA_BROKER_ADDR value
ASSERT output contains "50053" OR GRPC_PORT value
ASSERT output contains "ready"
```

### TS-08-21: Graceful Shutdown

**Requirement:** 08-REQ-8.3
**Type:** integration
**Description:** Verify the service exits cleanly on SIGTERM.

**Preconditions:**
- Service is running with active subscriptions.

**Input:**
- Send SIGTERM.

**Expected:**
- Process exits with code 0.

**Assertion pseudocode:**
```
proc = start_adaptor()
wait_for_ready()
send_signal(proc, SIGTERM)
exit_code = wait_for_exit(proc, timeout=5s)
ASSERT exit_code == 0
```

### TS-08-22: Session State Fields

**Requirement:** 08-REQ-6.1, 08-REQ-6.2, 08-REQ-6.3
**Type:** unit
**Description:** Verify session state is correctly populated and cleared.

**Preconditions:**
- None.

**Input:**
- Start session, then stop session.

**Expected:**
- After start: all fields populated, active=true.
- After stop: active=false, fields cleared.

**Assertion pseudocode:**
```
session = Session::new()
ASSERT session.is_active() == false

session.start("s1", "zone-a", 1700000000, Rate { rate_type: "per_hour", amount: 2.5, currency: "EUR" })
state = session.status()
ASSERT state.session_id == "s1"
ASSERT state.zone_id == "zone-a"
ASSERT state.start_time == 1700000000
ASSERT state.rate.rate_type == "per_hour"
ASSERT state.active == true

session.stop()
ASSERT session.is_active() == false
ASSERT session.status() == None
```

## Edge Case Tests

### TS-08-E1: StartSession When Already Active

**Requirement:** 08-REQ-1.E1
**Type:** unit
**Description:** Verify StartSession returns ALREADY_EXISTS when a session is active.

**Preconditions:**
- Active session with session_id="sess-1".

**Input:**
- gRPC StartSession(zone_id="zone-b").

**Expected:**
- gRPC error with code ALREADY_EXISTS.

**Assertion pseudocode:**
```
session.start("sess-1", "zone-a", now(), rate)
result = grpc_client.start_session("zone-b")
ASSERT result.is_err()
ASSERT result.error.code == ALREADY_EXISTS
```

### TS-08-E2: StopSession When No Session Active

**Requirement:** 08-REQ-1.E2
**Type:** unit
**Description:** Verify StopSession returns FAILED_PRECONDITION when no session.

**Preconditions:**
- No active session.

**Input:**
- gRPC StopSession().

**Expected:**
- gRPC error with code FAILED_PRECONDITION.

**Assertion pseudocode:**
```
result = grpc_client.stop_session()
ASSERT result.is_err()
ASSERT result.error.code == FAILED_PRECONDITION
```

### TS-08-E3: Operator REST Retry on Failure

**Requirement:** 08-REQ-2.E1
**Type:** unit
**Description:** Verify the adaptor retries operator REST calls with exponential backoff.

**Preconditions:**
- Mock HTTP server configured to fail first 2 calls, succeed on 3rd.

**Input:**
- operator.start_session("VIN", "zone").

**Expected:**
- 3 HTTP requests sent (with delays 1s, 2s between them).
- Final response parsed successfully.

**Assertion pseudocode:**
```
mock_server.fail_n_times(2)
mock_server.then_respond_with(start_response)
resp = client.start_session("VIN", "zone").await
ASSERT resp.is_ok()
ASSERT mock_server.request_count == 3
```

### TS-08-E4: Operator REST All Retries Exhausted

**Requirement:** 08-REQ-2.E1
**Type:** unit
**Description:** Verify the adaptor returns error after all retries fail.

**Preconditions:**
- Mock HTTP server configured to fail all requests.

**Input:**
- operator.start_session("VIN", "zone").

**Expected:**
- 4 HTTP requests sent (1 initial + 3 retries).
- Error returned. Session state not updated.

**Assertion pseudocode:**
```
mock_server.always_fail()
resp = client.start_session("VIN", "zone").await
ASSERT resp.is_err()
ASSERT mock_server.request_count == 4
ASSERT session.is_active() == false
```

### TS-08-E5: Operator Non-200 Status Triggers Retry

**Requirement:** 08-REQ-2.E2
**Type:** unit
**Description:** Verify non-200 HTTP responses trigger retry logic.

**Preconditions:**
- Mock HTTP server returns 500 twice, then 200.

**Input:**
- operator.start_session("VIN", "zone").

**Expected:**
- Retry applied for 500 responses.
- Final 200 response parsed successfully.

**Assertion pseudocode:**
```
mock_server.respond_sequence([500, 500, 200_with_body])
resp = client.start_session("VIN", "zone").await
ASSERT resp.is_ok()
ASSERT mock_server.request_count == 3
```

### TS-08-E6: Lock Event While Session Active (No-op)

**Requirement:** 08-REQ-3.E1
**Type:** unit
**Description:** Verify lock event during active session is a no-op.

**Preconditions:**
- Active session.

**Input:**
- Lock event: IsLocked=true.

**Expected:**
- Operator not called. Session unchanged. Info logged.

**Assertion pseudocode:**
```
session.start("sess-1", "zone-a", now(), rate)
process_lock_event(is_locked=true)
ASSERT mock_operator.start_call_count == 0
ASSERT session.is_active() == true
ASSERT session.status().session_id == "sess-1"
```

### TS-08-E7: Unlock Event While No Session (No-op)

**Requirement:** 08-REQ-3.E2
**Type:** unit
**Description:** Verify unlock event without active session is a no-op.

**Preconditions:**
- No active session.

**Input:**
- Unlock event: IsLocked=false.

**Expected:**
- Operator not called. Session unchanged. Info logged.

**Assertion pseudocode:**
```
process_lock_event(is_locked=false)
ASSERT mock_operator.stop_call_count == 0
ASSERT session.is_active() == false
```

### TS-08-E8: DATA_BROKER Unreachable on Startup

**Requirement:** 08-REQ-3.E3
**Type:** integration
**Description:** Verify retry behavior when DATA_BROKER is unreachable.

**Preconditions:**
- No DATA_BROKER running.

**Input:**
- Start adaptor with DATA_BROKER_ADDR pointing to non-listening port.

**Expected:**
- Service retries connection (logs indicate retries).
- Service exits with non-zero code after max retries.

**Assertion pseudocode:**
```
proc = start_adaptor(data_broker_addr="http://localhost:19999")
exit_code = wait_for_exit(proc, timeout=30s)
ASSERT exit_code != 0
ASSERT log_contains("retry")
```

### TS-08-E9: SessionActive Publish Failure

**Requirement:** 08-REQ-4.E1
**Type:** unit
**Description:** Verify the service continues after failing to publish SessionActive.

**Preconditions:**
- Mock broker configured to fail set_bool calls.

**Input:**
- Successful session start.

**Expected:**
- Session state updated in memory (active=true).
- Error logged for DATA_BROKER publish failure.
- Service continues to process events.

**Assertion pseudocode:**
```
mock_broker.fail_set_bool()
process_start_session()
ASSERT session.is_active() == true  // memory state still correct
// subsequent events still processed
process_lock_event(is_locked=false)  // should not panic
```

### TS-08-E10: GRPC_PORT Non-Numeric

**Requirement:** 08-REQ-7.E1
**Type:** unit
**Description:** Verify non-numeric GRPC_PORT causes exit.

**Preconditions:**
- None.

**Input:**
- GRPC_PORT="abc".

**Expected:**
- load_config returns error.

**Assertion pseudocode:**
```
set_env("GRPC_PORT", "abc")
result = load_config()
ASSERT result.is_err()
```

### TS-08-E11: Override Resumes Autonomous on Next Cycle

**Requirement:** 08-REQ-5.E1, 08-REQ-5.3
**Type:** unit
**Description:** Verify autonomous behavior resumes after manual override.

**Preconditions:**
- Active session started manually.

**Input:**
- Manual StopSession.
- Then lock event (IsLocked=true).

**Expected:**
- StopSession stops the session.
- Lock event starts a new session autonomously.

**Assertion pseudocode:**
```
// Manual start
grpc_client.start_session("zone-a")
ASSERT session.is_active() == true

// Manual stop
grpc_client.stop_session()
ASSERT session.is_active() == false

// Autonomous resumes
process_lock_event(is_locked=true)
ASSERT session.is_active() == true
ASSERT mock_operator.start_call_count == 2
```

### TS-08-E12: Service Restart Loses Session

**Requirement:** 08-REQ-6.E1
**Type:** integration
**Description:** Verify session state is lost on restart.

**Preconditions:**
- Adaptor running with active session.

**Input:**
- Restart the adaptor process.

**Expected:**
- GetStatus returns active=false after restart.

**Assertion pseudocode:**
```
start_adaptor()
trigger_lock_event()  // start session
resp = grpc_client.get_status()
ASSERT resp.active == true

restart_adaptor()
resp = grpc_client.get_status()
ASSERT resp.active == false
```

## Property Test Cases

### TS-08-P1: Session State Consistency

**Property:** Property 1 from design.md
**Validates:** 08-REQ-6.1, 08-REQ-6.2, 08-REQ-6.3
**Type:** property
**Description:** After any sequence of start/stop operations, session state matches the last successful operation.

**For any:** Sequence of (start, stop) operations with random success/failure outcomes
**Invariant:** session.is_active() matches the outcome of the last successful operation.

**Assertion pseudocode:**
```
FOR ANY ops IN arbitrary_sequence_of(Start, Stop), outcomes IN arbitrary_sequence_of(Ok, Err):
    session = Session::new()
    last_successful = None
    FOR (op, outcome) IN zip(ops, outcomes):
        IF outcome == Ok:
            IF op == Start:
                session.start(...)
                last_successful = Start
            ELSE:
                session.stop()
                last_successful = Stop
    IF last_successful == Start:
        ASSERT session.is_active() == true
    ELSE:
        ASSERT session.is_active() == false
```

### TS-08-P2: Idempotent Lock Events

**Property:** Property 2 from design.md
**Validates:** 08-REQ-3.E1, 08-REQ-3.E2
**Type:** property
**Description:** Duplicate lock/unlock events are no-ops.

**For any:** N consecutive lock events (N in 1..10) followed by M consecutive unlock events (M in 1..10)
**Invariant:** Operator start_session called exactly once, stop_session called exactly once.

**Assertion pseudocode:**
```
FOR ANY n IN 1..10, m IN 1..10:
    mock_operator.reset()
    FOR i IN 1..n:
        process_lock_event(is_locked=true)
    ASSERT mock_operator.start_call_count == 1
    FOR j IN 1..m:
        process_lock_event(is_locked=false)
    ASSERT mock_operator.stop_call_count == 1
```

### TS-08-P3: Override Non-Persistence

**Property:** Property 3 from design.md
**Validates:** 08-REQ-5.1, 08-REQ-5.2, 08-REQ-5.3, 08-REQ-5.E1
**Type:** property
**Description:** After any manual override, the next lock/unlock cycle resumes autonomous behavior.

**For any:** Manual start or stop, followed by a lock or unlock event
**Invariant:** The subsequent event triggers normal autonomous processing.

**Assertion pseudocode:**
```
FOR ANY override IN [ManualStart, ManualStop]:
    mock_operator.reset()
    IF override == ManualStart:
        grpc_client.start_session("zone")
    ELSE:
        session.start(...)
        grpc_client.stop_session()

    // Next cycle should be autonomous
    process_lock_event(is_locked=true)
    IF override == ManualStop:
        ASSERT mock_operator.start_call_count >= 1  // autonomous start
    process_lock_event(is_locked=false)
    ASSERT session.is_active() == false  // autonomous stop
```

### TS-08-P4: Retry Exhaustion Safety

**Property:** Property 4 from design.md
**Validates:** 08-REQ-2.E1, 08-REQ-2.E2
**Type:** property
**Description:** Failed REST calls never corrupt session state.

**For any:** Operator failure scenario (timeout, 500, connection refused)
**Invariant:** Session state before the failed operation equals session state after.

**Assertion pseudocode:**
```
FOR ANY failure_type IN [Timeout, ServerError, ConnectionRefused]:
    state_before = session.clone()
    mock_operator.always_fail_with(failure_type)
    process_lock_event(is_locked=true)
    ASSERT session == state_before  // state unchanged
```

### TS-08-P5: SessionActive Signal Consistency

**Property:** Property 5 from design.md
**Validates:** 08-REQ-4.1, 08-REQ-4.2, 08-REQ-4.3
**Type:** property
**Description:** SessionActive signal always matches session.active after successful operations.

**For any:** Sequence of successful start/stop operations
**Invariant:** Last set_bool call matches session.is_active().

**Assertion pseudocode:**
```
FOR ANY ops IN arbitrary_sequence_of(Start, Stop):
    mock_broker.reset()
    session = Session::new()
    FOR op IN ops:
        IF op == Start AND !session.is_active():
            process_start_session()
            ASSERT mock_broker.last_set_bool == ("Vehicle.Parking.SessionActive", true)
        ELSE IF op == Stop AND session.is_active():
            process_stop_session()
            ASSERT mock_broker.last_set_bool == ("Vehicle.Parking.SessionActive", false)
    ASSERT mock_broker.last_session_active_value == session.is_active()
```

### TS-08-P6: Sequential Event Processing

**Property:** Property 6 from design.md
**Validates:** 08-REQ-9.1, 08-REQ-9.2
**Type:** property
**Description:** No concurrent session state mutations occur.

**For any:** N events arriving simultaneously (N in 2..5)
**Invariant:** Each event is processed fully before the next begins. Final state is deterministic.

**Assertion pseudocode:**
```
FOR ANY events IN arbitrary_sequence_of(Lock, Unlock, ManualStart, ManualStop), len 2..5:
    result_sequential = process_all_sequentially(events)
    result_concurrent = process_all_concurrently(events)
    ASSERT result_sequential == result_concurrent  // deterministic
```

## Integration Smoke Tests

### TS-08-SMOKE-1: Lock-Start-Unlock-Stop Flow

**Type:** integration
**Description:** End-to-end: lock event triggers session start, unlock event triggers session stop.

**Preconditions:**
- DATA_BROKER container running.
- Mock PARKING_OPERATOR HTTP server running.
- Adaptor running with all connections established.

**Input:**
1. Set Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = true in DATA_BROKER.
2. Wait for operator start call.
3. Query GetStatus via gRPC.
4. Set Vehicle.Cabin.Door.Row1.DriverSide.IsLocked = false in DATA_BROKER.
5. Wait for operator stop call.
6. Query GetStatus via gRPC.

**Expected:**
1. Operator POST /parking/start called.
2. GetStatus returns active=true with session_id.
3. Vehicle.Parking.SessionActive = true in DATA_BROKER.
4. Operator POST /parking/stop called.
5. GetStatus returns active=false.
6. Vehicle.Parking.SessionActive = false in DATA_BROKER.

**Assertion pseudocode:**
```
start_adaptor()
start_mock_operator()

// Lock → start session
set_signal("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", true)
wait_for(mock_operator.start_called)
status = grpc_client.get_status()
ASSERT status.active == true
ASSERT status.session_id != ""
ASSERT get_signal("Vehicle.Parking.SessionActive") == true

// Unlock → stop session
set_signal("Vehicle.Cabin.Door.Row1.DriverSide.IsLocked", false)
wait_for(mock_operator.stop_called)
status = grpc_client.get_status()
ASSERT status.active == false
ASSERT get_signal("Vehicle.Parking.SessionActive") == false
```

### TS-08-SMOKE-2: Manual Override Flow

**Type:** integration
**Description:** End-to-end: manual StartSession and StopSession via gRPC.

**Preconditions:**
- DATA_BROKER container running.
- Mock PARKING_OPERATOR HTTP server running.
- Adaptor running.

**Input:**
1. gRPC StartSession(zone_id="zone-manual").
2. Query GetStatus.
3. gRPC StopSession().
4. Query GetStatus.

**Expected:**
1. Session starts, operator called.
2. GetStatus returns active=true.
3. Session stops, operator called.
4. GetStatus returns active=false.

**Assertion pseudocode:**
```
start_adaptor()
start_mock_operator()

resp = grpc_client.start_session("zone-manual")
ASSERT resp.session_id != ""
status = grpc_client.get_status()
ASSERT status.active == true

resp = grpc_client.stop_session()
ASSERT resp.session_id != ""
status = grpc_client.get_status()
ASSERT status.active == false
```

### TS-08-SMOKE-3: Override Then Autonomous Resume

**Type:** integration
**Description:** End-to-end: manual stop followed by autonomous lock/unlock cycle.

**Preconditions:**
- DATA_BROKER container running.
- Mock PARKING_OPERATOR HTTP server running.
- Adaptor running.

**Input:**
1. Lock event (start session autonomously).
2. gRPC StopSession (manual override).
3. Lock event (should start new session autonomously).
4. Unlock event (should stop session autonomously).

**Expected:**
1. Session active.
2. Session stopped (manual).
3. New session started (autonomous).
4. Session stopped (autonomous).

**Assertion pseudocode:**
```
set_signal("IsLocked", true)
wait_for(mock_operator.start_called)
ASSERT grpc_client.get_status().active == true

grpc_client.stop_session()
ASSERT grpc_client.get_status().active == false

set_signal("IsLocked", true)
wait_for(mock_operator.start_called, count=2)
ASSERT grpc_client.get_status().active == true

set_signal("IsLocked", false)
wait_for(mock_operator.stop_called, count=2)
ASSERT grpc_client.get_status().active == false
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 08-REQ-1.1 | TS-08-1 | unit |
| 08-REQ-1.2 | TS-08-2 | unit |
| 08-REQ-1.3 | TS-08-3 | unit |
| 08-REQ-1.4 | TS-08-4, TS-08-5 | unit |
| 08-REQ-1.5 | TS-08-6, TS-08-7 | unit |
| 08-REQ-1.E1 | TS-08-E1 | unit |
| 08-REQ-1.E2 | TS-08-E2 | unit |
| 08-REQ-2.1 | TS-08-8 | unit |
| 08-REQ-2.2 | TS-08-9 | unit |
| 08-REQ-2.3 | TS-08-10 | unit |
| 08-REQ-2.4 | TS-08-9 | unit |
| 08-REQ-2.E1 | TS-08-E3, TS-08-E4 | unit |
| 08-REQ-2.E2 | TS-08-E5 | unit |
| 08-REQ-3.1 | TS-08-11 | unit |
| 08-REQ-3.2 | TS-08-11 | unit |
| 08-REQ-3.3 | TS-08-11 | unit |
| 08-REQ-3.4 | TS-08-12 | unit |
| 08-REQ-3.E1 | TS-08-E6 | unit |
| 08-REQ-3.E2 | TS-08-E7 | unit |
| 08-REQ-3.E3 | TS-08-E8 | integration |
| 08-REQ-4.1 | TS-08-13 | unit |
| 08-REQ-4.2 | TS-08-14 | unit |
| 08-REQ-4.3 | TS-08-15 | integration |
| 08-REQ-4.E1 | TS-08-E9 | unit |
| 08-REQ-5.1 | TS-08-16 | unit |
| 08-REQ-5.2 | TS-08-17 | unit |
| 08-REQ-5.3 | TS-08-E11 | unit |
| 08-REQ-5.E1 | TS-08-E11 | unit |
| 08-REQ-6.1 | TS-08-22 | unit |
| 08-REQ-6.2 | TS-08-22 | unit |
| 08-REQ-6.3 | TS-08-22 | unit |
| 08-REQ-6.E1 | TS-08-E12 | integration |
| 08-REQ-7.1 | TS-08-18, TS-08-19 | unit |
| 08-REQ-7.2 | TS-08-18, TS-08-19 | unit |
| 08-REQ-7.3 | TS-08-18, TS-08-19 | unit |
| 08-REQ-7.4 | TS-08-18, TS-08-19 | unit |
| 08-REQ-7.5 | TS-08-18, TS-08-19 | unit |
| 08-REQ-7.E1 | TS-08-E10 | unit |
| 08-REQ-8.1 | TS-08-20 | integration |
| 08-REQ-8.2 | TS-08-20 | integration |
| 08-REQ-8.3 | TS-08-21 | integration |
| 08-REQ-8.E1 | TS-08-21 | integration |
| 08-REQ-9.1 | TS-08-P6 | property |
| 08-REQ-9.2 | TS-08-P6 | property |
| 08-REQ-9.E1 | TS-08-P6 | property |
| Property 1 | TS-08-P1 | property |
| Property 2 | TS-08-P2 | property |
| Property 3 | TS-08-P3 | property |
| Property 4 | TS-08-P4 | property |
| Property 5 | TS-08-P5 | property |
| Property 6 | TS-08-P6 | property |
