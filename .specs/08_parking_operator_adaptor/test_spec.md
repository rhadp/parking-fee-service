# Test Specification: PARKING_OPERATOR_ADAPTOR

## Overview

This test specification defines test contracts for the PARKING_OPERATOR_ADAPTOR. Tests use mock `OperatorClient` and `BrokerClient` trait implementations for unit testing without real PARKING_OPERATOR or DATA_BROKER dependencies. Unit tests run via `cd rhivos && cargo test -p parking-operator-adaptor`. Integration tests in `tests/parking-operator-adaptor/` test end-to-end gRPC behavior.

## Test Cases

### TS-08-1: Autonomous Start on Lock

**Requirement:** 08-REQ-1.1
**Type:** unit
**Description:** When IsLocked changes to true and no session is active, the adaptor calls the operator start API.

**Preconditions:**
- Mock OperatorClient: start_session returns success with session_id and rate.
- Mock BrokerClient: set_session_active succeeds.
- No active session.

**Input:**
- Lock event: IsLocked = true.

**Expected:**
- OperatorClient.start_session called with vehicle_id, zone_id.
- Session state updated.

**Assertion pseudocode:**
```
mock_operator.set_start_response(ok_response)
handle_lock_event(true, session_mgr, mock_operator, mock_broker)
ASSERT mock_operator.start_session_called
ASSERT session_mgr.is_active() == true
```

### TS-08-2: Session State Stored After Start

**Requirement:** 08-REQ-1.2
**Type:** unit
**Description:** After successful start, session_id, zone_id, start_time, and rate are stored.

**Preconditions:**
- Start succeeds with session_id="sess-001", rate={per_hour, 2.50, EUR}.

**Input:**
- Lock event triggers start.

**Expected:**
- session_mgr.get_status() returns stored state.

**Assertion pseudocode:**
```
handle_lock_event(true, ...)
status = session_mgr.get_status()
ASSERT status.session_id == "sess-001"
ASSERT status.rate.amount == 2.50
ASSERT status.active == true
```

### TS-08-3: SessionActive Written on Start

**Requirement:** 08-REQ-1.3
**Type:** unit
**Description:** After starting a session, SessionActive=true is written to DATA_BROKER.

**Preconditions:**
- Mock BrokerClient records set calls.

**Input:**
- Lock event triggers successful start.

**Expected:**
- broker.set_session_active(true) called.

**Assertion pseudocode:**
```
handle_lock_event(true, ...)
ASSERT mock_broker.set_session_active_called_with(true)
```

### TS-08-4: Autonomous Stop on Unlock

**Requirement:** 08-REQ-2.1
**Type:** unit
**Description:** When IsLocked changes to false and a session is active, the adaptor calls operator stop API.

**Preconditions:**
- Active session with session_id="sess-001".
- Mock OperatorClient: stop_session succeeds.

**Input:**
- Unlock event: IsLocked = false.

**Expected:**
- OperatorClient.stop_session called with session_id.

**Assertion pseudocode:**
```
session_mgr.start("sess-001", "zone-1", rate)
handle_lock_event(false, session_mgr, mock_operator, mock_broker)
ASSERT mock_operator.stop_session_called_with("sess-001")
```

### TS-08-5: Session Cleared After Stop

**Requirement:** 08-REQ-2.2
**Type:** unit
**Description:** After successful stop, session state is cleared.

**Preconditions:**
- Active session.

**Input:**
- Unlock event triggers stop.

**Expected:**
- session_mgr.is_active() == false.

**Assertion pseudocode:**
```
handle_lock_event(false, ...)
ASSERT session_mgr.is_active() == false
```

### TS-08-6: SessionActive Written on Stop

**Requirement:** 08-REQ-2.3
**Type:** unit
**Description:** After stopping a session, SessionActive=false is written to DATA_BROKER.

**Preconditions:**
- Mock BrokerClient records calls.

**Input:**
- Unlock event triggers successful stop.

**Expected:**
- broker.set_session_active(false) called.

**Assertion pseudocode:**
```
handle_lock_event(false, ...)
ASSERT mock_broker.set_session_active_called_with(false)
```

### TS-08-7: Manual StartSession

**Requirement:** 08-REQ-3.1
**Type:** unit
**Description:** StartSession gRPC call starts a session via operator API.

**Preconditions:**
- No active session. Mock operator succeeds.

**Input:**
- StartSession(zone_id="zone-demo-1").

**Expected:**
- Operator start called. Session active.

**Assertion pseudocode:**
```
result = grpc_service.start_session("zone-demo-1")
ASSERT result.is_ok()
ASSERT session_mgr.is_active() == true
```

### TS-08-8: Manual StopSession

**Requirement:** 08-REQ-3.2
**Type:** unit
**Description:** StopSession gRPC call stops session regardless of lock state.

**Preconditions:**
- Active session.

**Input:**
- StopSession().

**Expected:**
- Operator stop called. Session inactive.

**Assertion pseudocode:**
```
result = grpc_service.stop_session()
ASSERT result.is_ok()
ASSERT session_mgr.is_active() == false
```

### TS-08-9: Resume Autonomous After Override

**Requirement:** 08-REQ-3.3
**Type:** unit
**Description:** After manual stop, autonomous behavior resumes on next lock event.

**Preconditions:**
- Manual stop was called. Lock state is true.

**Input:**
- Lock event (IsLocked=false then true again).

**Expected:**
- New session started autonomously.

**Assertion pseudocode:**
```
grpc_service.stop_session()
handle_lock_event(false, ...)  // unlock
handle_lock_event(true, ...)   // re-lock
ASSERT session_mgr.is_active() == true
```

### TS-08-10: GetStatus Active Session

**Requirement:** 08-REQ-4.1
**Type:** unit
**Description:** GetStatus returns session info when active.

**Preconditions:**
- Active session.

**Input:**
- GetStatus().

**Expected:**
- Returns session_id, active=true, zone_id, start_time, rate.

**Assertion pseudocode:**
```
session_mgr.start("sess-001", "zone-1", rate)
status = session_mgr.get_status()
ASSERT status.session_id == "sess-001"
ASSERT status.active == true
```

### TS-08-11: GetStatus No Session

**Requirement:** 08-REQ-4.2
**Type:** unit
**Description:** GetStatus returns active=false when no session.

**Preconditions:**
- No active session.

**Input:**
- GetStatus().

**Expected:**
- Returns None or active=false.

**Assertion pseudocode:**
```
status = session_mgr.get_status()
ASSERT status == None
```

### TS-08-12: GetRate Active Session

**Requirement:** 08-REQ-5.1
**Type:** unit
**Description:** GetRate returns cached rate when session active.

**Preconditions:**
- Active session with rate {per_hour, 2.50, EUR}.

**Input:**
- GetRate().

**Expected:**
- Returns rate info.

**Assertion pseudocode:**
```
session_mgr.start("sess-001", "zone-1", Rate{per_hour, 2.50, EUR})
rate = session_mgr.get_rate()
ASSERT rate.rate_type == "per_hour"
ASSERT rate.amount == 2.50
```

### TS-08-13: Lock Subscription

**Requirement:** 08-REQ-6.1
**Type:** unit
**Description:** On startup, the adaptor subscribes to IsLocked signal.

**Preconditions:**
- Mock BrokerClient.

**Input:**
- Start auto-session loop.

**Expected:**
- broker.subscribe_lock_state() called.

**Assertion pseudocode:**
```
start_auto_loop(mock_broker, ...)
ASSERT mock_broker.subscribe_lock_state_called
```

### TS-08-14: SessionActive Written

**Requirement:** 08-REQ-6.2
**Type:** unit
**Description:** Session start/stop writes SessionActive to DATA_BROKER.

**Preconditions:**
- Mock BrokerClient.

**Input:**
- Start then stop a session.

**Expected:**
- set_session_active(true) then set_session_active(false) called.

**Assertion pseudocode:**
```
// Covered by TS-08-3 and TS-08-6
```

### TS-08-15: Config from Env Vars

**Requirement:** 08-REQ-7.1
**Type:** unit
**Description:** Config reads from environment variables.

**Preconditions:**
- Set env vars.

**Input:**
- PARKING_OPERATOR_URL=http://op:9090, GRPC_PORT=50099.

**Expected:**
- Config reflects env values.

**Assertion pseudocode:**
```
set_env("PARKING_OPERATOR_URL", "http://op:9090")
set_env("GRPC_PORT", "50099")
cfg = load_config()
ASSERT cfg.parking_operator_url == "http://op:9090"
ASSERT cfg.grpc_port == 50099
```

### TS-08-16: Config Defaults

**Requirement:** 08-REQ-7.2
**Type:** unit
**Description:** Missing env vars use defaults.

**Preconditions:**
- No env vars set.

**Input:**
- load_config().

**Expected:**
- Defaults applied.

**Assertion pseudocode:**
```
cfg = load_config()
ASSERT cfg.parking_operator_url == "http://localhost:8080"
ASSERT cfg.data_broker_addr == "http://localhost:55556"
ASSERT cfg.grpc_port == 50053
ASSERT cfg.vehicle_id == "DEMO-VIN-001"
ASSERT cfg.zone_id == "zone-demo-1"
```

### TS-08-17: Startup Logging

**Requirement:** 08-REQ-8.1
**Type:** integration
**Description:** On startup, the adaptor logs version, port, operator URL, DATA_BROKER addr.

**Preconditions:**
- Service starts.

**Input:**
- Capture startup logs.

**Expected:**
- Logs contain port, operator URL.

**Assertion pseudocode:**
```
output = captureStartupLogs()
ASSERT "50053" IN output
ASSERT "operator" IN output
```

### TS-08-18: Graceful Shutdown

**Requirement:** 08-REQ-8.2
**Type:** integration
**Description:** SIGTERM stops active session and exits with code 0.

**Preconditions:**
- Service running.

**Input:**
- Send SIGTERM.

**Expected:**
- Exit code 0.

**Assertion pseudocode:**
```
proc = startService()
proc.Signal(SIGTERM)
exitCode = proc.Wait()
ASSERT exitCode == 0
```

## Edge Case Tests

### TS-08-E1: Lock While Session Active

**Requirement:** 08-REQ-1.E1
**Type:** unit
**Description:** Lock event while session active is a no-op.

**Preconditions:**
- Active session.

**Input:**
- Lock event (IsLocked=true).

**Expected:**
- Operator start NOT called. Session unchanged.

**Assertion pseudocode:**
```
session_mgr.start("sess-001", "zone-1", rate)
mock_operator.reset_call_counts()
handle_lock_event(true, ...)
ASSERT mock_operator.start_session_not_called
ASSERT session_mgr.get_status().session_id == "sess-001"
```

### TS-08-E2: Operator Start Failure

**Requirement:** 08-REQ-1.E2
**Type:** unit
**Description:** Start failure after 3 retries logs error, no session created.

**Preconditions:**
- Mock OperatorClient: start_session always fails.

**Input:**
- Lock event.

**Expected:**
- 3 attempts made. Session not active.

**Assertion pseudocode:**
```
mock_operator.set_start_error(true)
handle_lock_event(true, ...)
ASSERT mock_operator.start_call_count == 3
ASSERT session_mgr.is_active() == false
```

### TS-08-E3: Unlock While No Session

**Requirement:** 08-REQ-2.E1
**Type:** unit
**Description:** Unlock event while no session is a no-op.

**Preconditions:**
- No active session.

**Input:**
- Unlock event (IsLocked=false).

**Expected:**
- Operator stop NOT called.

**Assertion pseudocode:**
```
handle_lock_event(false, ...)
ASSERT mock_operator.stop_session_not_called
```

### TS-08-E4: Operator Stop Failure

**Requirement:** 08-REQ-2.E2
**Type:** unit
**Description:** Stop failure after 3 retries logs error, session not cleared.

**Preconditions:**
- Active session. Mock OperatorClient: stop_session always fails.

**Input:**
- Unlock event.

**Expected:**
- 3 attempts made. Session still active.

**Assertion pseudocode:**
```
mock_operator.set_stop_error(true)
session_mgr.start("sess-001", ...)
handle_lock_event(false, ...)
ASSERT mock_operator.stop_call_count == 3
ASSERT session_mgr.is_active() == true
```

### TS-08-E5: StartSession While Active

**Requirement:** 08-REQ-3.E1
**Type:** unit
**Description:** Manual StartSession while session active returns ALREADY_EXISTS.

**Preconditions:**
- Active session.

**Input:**
- StartSession("zone-1").

**Expected:**
- gRPC ALREADY_EXISTS error.

**Assertion pseudocode:**
```
session_mgr.start("sess-001", ...)
err = grpc_service.start_session("zone-1")
ASSERT err.code == ALREADY_EXISTS
```

### TS-08-E6: StopSession While No Session

**Requirement:** 08-REQ-3.E2
**Type:** unit
**Description:** Manual StopSession while no session returns NOT_FOUND.

**Preconditions:**
- No active session.

**Input:**
- StopSession().

**Expected:**
- gRPC NOT_FOUND error.

**Assertion pseudocode:**
```
err = grpc_service.stop_session()
ASSERT err.code == NOT_FOUND
```

### TS-08-E7: GetRate No Session

**Requirement:** 08-REQ-5.2
**Type:** unit
**Description:** GetRate with no session returns NOT_FOUND.

**Preconditions:**
- No active session.

**Input:**
- GetRate().

**Expected:**
- gRPC NOT_FOUND.

**Assertion pseudocode:**
```
rate = session_mgr.get_rate()
ASSERT rate == None
```

### TS-08-E8: DATA_BROKER Unreachable

**Requirement:** 08-REQ-6.E1
**Type:** integration
**Description:** DATA_BROKER unreachable at startup causes retry then exit.

**Preconditions:**
- No DATA_BROKER running.

**Input:**
- Start service with unreachable DATA_BROKER_ADDR.

**Expected:**
- Service exits non-zero after retries.

**Assertion pseudocode:**
```
proc = startService(DATA_BROKER_ADDR="http://localhost:19999")
exitCode = proc.Wait()
ASSERT exitCode != 0
```

### TS-08-E9: SessionActive Write Failure

**Requirement:** 08-REQ-6.E2
**Type:** unit
**Description:** SessionActive write failure is logged but session continues.

**Preconditions:**
- Mock BrokerClient: set_session_active fails.

**Input:**
- Lock event triggers start.

**Expected:**
- Session started. Error logged. Service continues.

**Assertion pseudocode:**
```
mock_broker.set_session_active_error(true)
handle_lock_event(true, ...)
ASSERT session_mgr.is_active() == true
// broker error logged but not fatal
```

## Property Test Cases

### TS-08-P1: Autonomous Start on Lock

**Property:** Property 1 from design.md
**Validates:** 08-REQ-1.1, 08-REQ-1.2, 08-REQ-1.3
**Type:** property
**Description:** For any lock event when no session is active, start is called and state is updated.

**For any:** Random zone_id, vehicle_id, rate.
**Invariant:** After lock event with no active session, session is active and broker wrote true.

**Assertion pseudocode:**
```
FOR ANY zone_id IN random_strings, rate IN random_rates:
    ASSERT NOT session_mgr.is_active()
    handle_lock_event(true, ...)
    ASSERT session_mgr.is_active()
```

### TS-08-P2: Autonomous Stop on Unlock

**Property:** Property 2 from design.md
**Validates:** 08-REQ-2.1, 08-REQ-2.2, 08-REQ-2.3
**Type:** property
**Description:** For any unlock event when session is active, stop is called and state is cleared.

**For any:** Random active session states.
**Invariant:** After unlock with active session, session is inactive.

**Assertion pseudocode:**
```
FOR ANY session IN random_sessions:
    session_mgr.start(session.id, session.zone, session.rate)
    handle_lock_event(false, ...)
    ASSERT NOT session_mgr.is_active()
```

### TS-08-P3: Session Idempotency

**Property:** Property 3 from design.md
**Validates:** 08-REQ-1.E1, 08-REQ-2.E1
**Type:** property
**Description:** Lock while active or unlock while inactive = no operator calls.

**For any:** Random sequences of lock/unlock events.
**Invariant:** Duplicate events don't trigger operator calls.

**Assertion pseudocode:**
```
FOR ANY events IN random_bool_sequences:
    FOR event IN events:
        was_active = session_mgr.is_active()
        handle_lock_event(event, ...)
        IF event == true AND was_active:
            ASSERT mock_operator.start_not_called_this_round
        IF event == false AND NOT was_active:
            ASSERT mock_operator.stop_not_called_this_round
```

### TS-08-P4: Manual Override Consistency

**Property:** Property 4 from design.md
**Validates:** 08-REQ-3.1, 08-REQ-3.2, 08-REQ-3.3
**Type:** property
**Description:** Manual start/stop follows same logic as autonomous, and autonomous resumes.

**For any:** Random manual override sequences followed by lock events.
**Invariant:** After manual override, next lock/unlock cycle works autonomously.

**Assertion pseudocode:**
```
FOR ANY override IN [start, stop]:
    apply_override(override)
    // Simulate lock/unlock cycle
    handle_lock_event(false, ...)
    handle_lock_event(true, ...)
    ASSERT session_mgr.is_active() == true
```

### TS-08-P5: Operator Retry Logic

**Property:** Property 5 from design.md
**Validates:** 08-REQ-1.E2, 08-REQ-2.E2
**Type:** property
**Description:** Failures always result in exactly 3 retry attempts.

**For any:** Random failure scenarios (start or stop).
**Invariant:** Exactly 3 attempts are made.

**Assertion pseudocode:**
```
FOR ANY operation IN [start, stop]:
    mock_operator.set_always_fail(true)
    trigger_operation(operation)
    ASSERT mock_operator.call_count == 3
```

### TS-08-P6: Config Defaults

**Property:** Property 6 from design.md
**Validates:** 08-REQ-7.1, 08-REQ-7.2
**Type:** property
**Description:** Missing env vars always use defined defaults.

**For any:** Random subsets of env vars being set.
**Invariant:** Unset vars use defaults, set vars use provided values.

**Assertion pseudocode:**
```
FOR ANY subset IN random_env_subsets:
    clear_all_env()
    set_env_vars(subset)
    cfg = load_config()
    FOR var IN all_vars:
        IF var IN subset:
            ASSERT cfg[var] == subset[var]
        ELSE:
            ASSERT cfg[var] == default[var]
```

## Coverage Matrix

| Requirement | Test Spec Entry | Type |
|-------------|-----------------|------|
| 08-REQ-1.1 | TS-08-1 | unit |
| 08-REQ-1.2 | TS-08-2 | unit |
| 08-REQ-1.3 | TS-08-3 | unit |
| 08-REQ-1.E1 | TS-08-E1 | unit |
| 08-REQ-1.E2 | TS-08-E2 | unit |
| 08-REQ-2.1 | TS-08-4 | unit |
| 08-REQ-2.2 | TS-08-5 | unit |
| 08-REQ-2.3 | TS-08-6 | unit |
| 08-REQ-2.E1 | TS-08-E3 | unit |
| 08-REQ-2.E2 | TS-08-E4 | unit |
| 08-REQ-3.1 | TS-08-7 | unit |
| 08-REQ-3.2 | TS-08-8 | unit |
| 08-REQ-3.3 | TS-08-9 | unit |
| 08-REQ-3.E1 | TS-08-E5 | unit |
| 08-REQ-3.E2 | TS-08-E6 | unit |
| 08-REQ-4.1 | TS-08-10 | unit |
| 08-REQ-4.2 | TS-08-11 | unit |
| 08-REQ-5.1 | TS-08-12 | unit |
| 08-REQ-5.2 | TS-08-E7 | unit |
| 08-REQ-6.1 | TS-08-13 | unit |
| 08-REQ-6.2 | TS-08-14 | unit |
| 08-REQ-6.E1 | TS-08-E8 | integration |
| 08-REQ-6.E2 | TS-08-E9 | unit |
| 08-REQ-7.1 | TS-08-15 | unit |
| 08-REQ-7.2 | TS-08-16 | unit |
| 08-REQ-8.1 | TS-08-17 | integration |
| 08-REQ-8.2 | TS-08-18 | integration |
| Property 1 | TS-08-P1 | property |
| Property 2 | TS-08-P2 | property |
| Property 3 | TS-08-P3 | property |
| Property 4 | TS-08-P4 | property |
| Property 5 | TS-08-P5 | property |
| Property 6 | TS-08-P6 | property |
