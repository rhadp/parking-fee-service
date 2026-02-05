# Communication Cohesion Analysis

## Executive Summary

This document analyzes the inter-component communication patterns in the SDV Parking Demo System, identifying inconsistencies, gaps, and potential issues in messaging between components. The analysis covers protocol choices, message formats, field naming conventions, error handling, and API contract alignment.

---

## 1. Component Communication Map

### 1.1 Communication Protocols Overview

| From | To | Protocol | Transport | Format |
|------|------|----------|-----------|--------|
| PARKING_APP | DATA_BROKER | gRPC | TLS/TCP | Protobuf |
| PARKING_APP | UPDATE_SERVICE | gRPC | TLS/TCP | Protobuf |
| PARKING_APP | PARKING_OPERATOR_ADAPTOR | gRPC | TLS/TCP | Protobuf |
| PARKING_APP | PARKING_FEE_SERVICE | REST | HTTPS | JSON |
| PARKING_OPERATOR_ADAPTOR | DATA_BROKER | gRPC | UDS | Protobuf |
| PARKING_OPERATOR_ADAPTOR | PARKING_OPERATOR (external) | REST | HTTPS | JSON |
| UPDATE_SERVICE | REGISTRY | OCI | HTTPS | OCI Manifest |
| LOCKING_SERVICE | DATA_BROKER | gRPC | UDS | Protobuf |
| CLOUD_GATEWAY_CLIENT | LOCKING_SERVICE | gRPC | UDS | Protobuf |
| CLOUD_GATEWAY_CLIENT | DATA_BROKER | gRPC | UDS | Protobuf |
| CLOUD_GATEWAY_CLIENT | CLOUD_GATEWAY | MQTT | TLS/TCP | JSON |
| CLOUD_GATEWAY | COMPANION_APP | REST | HTTPS | JSON |
| COMPANION_APP | CLOUD_GATEWAY | REST | HTTPS | JSON |

---

## 2. Identified Communication Inconsistencies

### 2.1 MQTT Topic Structure

**Location**: CLOUD_GATEWAY_CLIENT ↔ CLOUD_GATEWAY

**Topics Used**:
```
vehicles/{VIN}/commands          - Lock/unlock commands to vehicle
vehicles/{VIN}/command_responses - Command results from vehicle
vehicles/{VIN}/telemetry         - Vehicle state updates
```

**Issue #1: Missing Topic for Parking Session Events**

The MQTT topics do not include a dedicated channel for parking session notifications. When a parking session starts or ends, there's no MQTT message sent to the cloud. The `parking_session_active` field is embedded in telemetry.

**Impact**: CLOUD_GATEWAY and COMPANION_APP cannot receive real-time parking session start/stop events directly; they must poll telemetry.

**Recommendation**: Add a dedicated topic `vehicles/{VIN}/parking_events` or ensure telemetry is pushed immediately on session state changes.

---

### 2.2 Command Message Field Naming

**Location**: CLOUD_GATEWAY ↔ CLOUD_GATEWAY_CLIENT (MQTT)

**CLOUD_GATEWAY publishes**:
```json
{
  "command_id": "cmd-abc123",
  "type": "lock",
  "doors": ["driver"],
  "auth_token": "token",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

**CLOUD_GATEWAY_CLIENT expects** (from design.md):
```json
{
  "command_id": "uuid-string",
  "type": "lock | unlock",
  "doors": ["driver", "passenger", "rear_left", "rear_right", "all"],
  "auth_token": "token-string"
}
```

**Issue #2: Missing `timestamp` in CLOUD_GATEWAY_CLIENT Documentation**

The CLOUD_GATEWAY includes `timestamp` in the MQTT command message, but CLOUD_GATEWAY_CLIENT's expected format doesn't explicitly list it.

**Impact**: Potential field being ignored or validation failure.

**Recommendation**: Align documentation - add `timestamp` to CLOUD_GATEWAY_CLIENT's expected command format.

---

### 2.3 Command Response Message Structure

**CLOUD_GATEWAY expects** (from MQTT subscription):
```json
{
  "command_id": "cmd-abc123",
  "status": "success" | "failed",
  "error_code": "DOOR_BLOCKED",
  "error_message": "Door is blocked",
  "timestamp": "2024-01-15T10:30:02Z"
}
```

**CLOUD_GATEWAY_CLIENT sends** (from design.md):
```json
{
  "command_id": "uuid-string",
  "status": "success | failed",
  "error_code": "optional-error-code",
  "error_message": "optional-error-description"
}
```

**Issue #3: Missing `timestamp` in CLOUD_GATEWAY_CLIENT Response**

CLOUD_GATEWAY expects `timestamp` but CLOUD_GATEWAY_CLIENT's documented format omits it.

**Impact**: CLOUD_GATEWAY may receive responses without timestamps for audit/logging.

**Recommendation**: Add `timestamp` to CLOUD_GATEWAY_CLIENT response format.

---

### 2.4 Telemetry Message Structure

**CLOUD_GATEWAY_CLIENT sends**:
```json
{
  "timestamp": "ISO8601-datetime",
  "location": {
    "latitude": 0.0,
    "longitude": 0.0
  },
  "door_locked": true,
  "door_open": false,
  "parking_session_active": false
}
```

**CLOUD_GATEWAY expects** (from design.md):
```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "latitude": 37.7749,
  "longitude": -122.4194,
  "door_locked": true,
  "door_open": false,
  "parking_session_active": true
}
```

**Issue #4: Nested `location` Object vs Flat Structure**

CLOUD_GATEWAY_CLIENT uses a nested `location` object, but CLOUD_GATEWAY expects flat `latitude`/`longitude` fields.

**Impact**: JSON parsing will fail or return null for location fields.

**Severity**: HIGH - Communication will be broken.

**Recommendation**: Align on one format. Prefer flat structure for consistency with other APIs.

---

### 2.5 gRPC Session State Enum Values

**PARKING_OPERATOR_ADAPTOR proto**:
```protobuf
enum SessionState {
  SESSION_STATE_NONE = 0;
  SESSION_STATE_STARTING = 1;
  SESSION_STATE_ACTIVE = 2;
  SESSION_STATE_STOPPING = 3;
  SESSION_STATE_STOPPED = 4;
  SESSION_STATE_ERROR = 5;
}
```

**PARKING_APP Kotlin mapping**:
```kotlin
enum class SessionState {
    NONE,
    STARTING,
    ACTIVE,
    STOPPING,
    STOPPED,
    ERROR
}
```

**Issue #5: Kotlin Enum Mapping Missing Prefix Handling**

The proto uses `SESSION_STATE_` prefix, but the Kotlin enum doesn't. The mapping code handles this, but there's no documentation on handling `UNRECOGNIZED` values.

**Recommendation**: Add explicit handling for unknown enum values:
```kotlin
else -> SessionState.NONE // or throw
```

---

### 2.6 UPDATE_SERVICE Adapter State Enum

**UPDATE_SERVICE proto**:
```protobuf
enum AdapterState {
  ADAPTER_STATE_UNKNOWN = 0;
  ADAPTER_STATE_DOWNLOADING = 1;
  ADAPTER_STATE_INSTALLING = 2;
  ADAPTER_STATE_RUNNING = 3;
  ADAPTER_STATE_STOPPED = 4;
  ADAPTER_STATE_ERROR = 5;
}
```

**PARKING_APP Kotlin mapping**:
```kotlin
enum class AdapterStatus {
    NOT_INSTALLED,
    INSTALLING,
    INSTALLED,
    ERROR
}
```

**Issue #6: State Enum Mismatch**

UPDATE_SERVICE uses: `UNKNOWN, DOWNLOADING, INSTALLING, RUNNING, STOPPED, ERROR`
PARKING_APP uses: `NOT_INSTALLED, INSTALLING, INSTALLED, ERROR`

Missing states in PARKING_APP:
- `DOWNLOADING` (mapped to `INSTALLING`?)
- `RUNNING` vs `INSTALLED`
- `STOPPED`
- `UNKNOWN` vs `NOT_INSTALLED`

**Impact**: PARKING_APP cannot accurately represent all adapter states.

**Recommendation**: Align PARKING_APP enum with UPDATE_SERVICE or document explicit mapping.

---

### 2.7 Door Enum Values

**LOCKING_SERVICE proto**:
```protobuf
enum Door {
  DOOR_UNKNOWN = 0;
  DOOR_DRIVER = 1;
  DOOR_PASSENGER = 2;
  DOOR_REAR_LEFT = 3;
  DOOR_REAR_RIGHT = 4;
  DOOR_ALL = 5;
}
```

**CLOUD_GATEWAY_CLIENT JSON**:
```json
"doors": ["driver", "passenger", "rear_left", "rear_right", "all"]
```

**CLOUD_GATEWAY REST API**:
```json
"doors": ["driver"] | ["all"]
```

**Issue #7: CLOUD_GATEWAY Only Supports Two Door Values**

LOCKING_SERVICE supports 5 door values, CLOUD_GATEWAY_CLIENT supports 5, but CLOUD_GATEWAY REST API only validates `driver` and `all`.

**Impact**: COMPANION_APP cannot lock individual passenger/rear doors.

**Recommendation**: Either:
- Extend CLOUD_GATEWAY to support all door values, OR
- Document that only `driver` and `all` are supported for demo scope

---

### 2.8 Zone Lookup API

**PARKING_APP calls** (expected):
```
GET /zones?lat={latitude}&lng={longitude}
```

**PARKING_FEE_SERVICE provides**:
```
GET /api/v1/zones?lat={latitude}&lng={longitude}
```

**Issue #8: Missing `/api/v1` Prefix in PARKING_APP Documentation**

PARKING_APP's ParkingFeeServiceClient doesn't specify the full path.

**Recommendation**: Ensure PARKING_APP configuration includes the full base URL with `/api/v1` prefix.

---

### 2.9 Zone Response Field Naming

**PARKING_FEE_SERVICE returns**:
```json
{
  "zone_id": "demo-zone-001",
  "operator_name": "Demo Parking Operator",
  "hourly_rate": 2.50,
  "currency": "USD",
  "adapter_image_ref": "...",
  "adapter_checksum": "sha256:..."
}
```

**PARKING_APP expects** (ZoneResponse):
```kotlin
@SerialName("zone_id") val zoneId: String,
@SerialName("operator_name") val operatorName: String,
@SerialName("hourly_rate") val hourlyRate: Double,
@SerialName("currency") val currency: String,
@SerialName("adapter_image_ref") val adapterImageRef: String,
@SerialName("adapter_checksum") val adapterChecksum: String
```

**Status**: Fields align correctly with `@SerialName` annotations.

---

### 2.10 Parking Session Start Request

**PARKING_OPERATOR_ADAPTOR calls PARKING_FEE_SERVICE**:
```json
{
  "vehicle_id": "string",
  "latitude": number,
  "longitude": number,
  "zone_id": "string",
  "timestamp": "ISO8601"
}
```

**PARKING_FEE_SERVICE expects**:
```json
{
  "vehicle_id": "string",
  "latitude": float64,
  "longitude": float64,
  "zone_id": "string",
  "timestamp": "string"
}
```

**Status**: Fields align correctly.

**Note**: The flow diagram in PARKING_OPERATOR_ADAPTOR design shows:
> "Zone Lookup: PARKING_APP queries PARKING_FEE_SERVICE for Zone_ID based on location"

**Issue #9: Zone Lookup Responsibility Unclear**

The design says PARKING_APP performs zone lookup, but PARKING_OPERATOR_ADAPTOR's StartSessionRequest already includes zone_id. Who determines the zone?

**Resolution from design docs**: PARKING_APP looks up zone and passes zone_id to PARKING_OPERATOR_ADAPTOR in StartSessionRequest.

---

### 2.11 StartSession gRPC Request

**PARKING_OPERATOR_ADAPTOR proto**:
```protobuf
message StartSessionRequest {
  string zone_id = 1;  // Zone_ID provided by PARKING_APP
}
```

**Issue #10: No Automatic Zone Determination on Lock Event**

When vehicle locks automatically (via DATA_BROKER signal), who provides zone_id? The automatic flow doesn't involve PARKING_APP.

**Impact**: Automatic session start on lock may fail without zone_id.

**Recommendation**:
- PARKING_OPERATOR_ADAPTOR should call PARKING_FEE_SERVICE directly for zone lookup during automatic lock events, OR
- Add zone_id lookup to PARKING_OPERATOR_ADAPTOR for automatic sessions

---

### 2.12 Error Code Consistency

**CLOUD_GATEWAY Error Codes**:
```go
ErrInvalidCommandType = "INVALID_COMMAND_TYPE"
ErrInvalidDoor        = "INVALID_DOOR"
ErrMissingAuthToken   = "MISSING_AUTH_TOKEN"
ErrVehicleNotFound    = "VEHICLE_NOT_FOUND"
ErrCommandNotFound    = "COMMAND_NOT_FOUND"
ErrTimeout            = "TIMEOUT"
```

**PARKING_FEE_SERVICE Error Codes**:
```go
ErrInvalidParameters = "INVALID_PARAMETERS"
ErrZoneNotFound      = "ZONE_NOT_FOUND"
ErrAdapterNotFound   = "ADAPTER_NOT_FOUND"
ErrSessionNotFound   = "SESSION_NOT_FOUND"
ErrValidationError   = "VALIDATION_ERROR"
```

**CLOUD_GATEWAY_CLIENT Error Codes** (response format):
```
MALFORMED_JSON, MISSING_FIELD, AUTH_FAILED, INVALID_COMMAND_TYPE,
INVALID_DOOR, SERVICE_UNAVAILABLE, EXECUTION_FAILED, TIMEOUT
```

**Issue #11: Inconsistent Error Code Naming Patterns**

- Some use `ERR_` prefix (Go constants), others don't (JSON values)
- Some use SCREAMING_SNAKE_CASE, others use different styles
- No shared error code enum across services

**Recommendation**: Create a shared error code specification document.

---

### 2.13 gRPC Status Code Usage

**PARKING_OPERATOR_ADAPTOR**:
| Error | gRPC Status |
|-------|-------------|
| Session already active | ALREADY_EXISTS (6) |
| No active session | NOT_FOUND (5) |
| Operation in progress | FAILED_PRECONDITION (9) |
| Location unavailable | FAILED_PRECONDITION (9) |
| Operator API error | UNAVAILABLE (14) |
| Timeout | DEADLINE_EXCEEDED (4) |

**UPDATE_SERVICE**:
| Error | gRPC Status |
|-------|-------------|
| Invalid registry URL | INVALID_ARGUMENT (3) |
| Registry unreachable | UNAVAILABLE (14) |
| Adapter not found | NOT_FOUND (5) |
| Adapter already exists | ALREADY_EXISTS (6) |
| Authentication failed | UNAUTHENTICATED (16) |

**LOCKING_SERVICE**:
| Error | gRPC Status |
|-------|-------------|
| Invalid auth token | UNAUTHENTICATED (16) |
| Door is open | FAILED_PRECONDITION (9) |
| Vehicle moving | FAILED_PRECONDITION (9) |
| Invalid door | INVALID_ARGUMENT (3) |
| Timeout | DEADLINE_EXCEEDED (4) |

**Status**: Generally consistent use of gRPC status codes.

---

### 2.14 VSS Signal Path Consistency

**Signals used across components**:

| Signal | Used By | Path |
|--------|---------|------|
| Door Lock State | PARKING_OPERATOR_ADAPTOR, CLOUD_GATEWAY_CLIENT, LOCKING_SERVICE | `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` |
| Door Open State | LOCKING_SERVICE, CLOUD_GATEWAY_CLIENT | `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` |
| Vehicle Speed | LOCKING_SERVICE | `Vehicle.Speed` |
| Latitude | PARKING_OPERATOR_ADAPTOR, CLOUD_GATEWAY_CLIENT, PARKING_APP | `Vehicle.CurrentLocation.Latitude` |
| Longitude | PARKING_OPERATOR_ADAPTOR, CLOUD_GATEWAY_CLIENT, PARKING_APP | `Vehicle.CurrentLocation.Longitude` |
| Session Active | PARKING_OPERATOR_ADAPTOR, PARKING_APP, CLOUD_GATEWAY_CLIENT | `Vehicle.Parking.SessionActive` |

**Status**: VSS signal paths are consistent across all components.

---

### 2.15 Timestamp Format

**Components using ISO8601**:
- CLOUD_GATEWAY (JSON REST API)
- CLOUD_GATEWAY_CLIENT (MQTT messages)
- PARKING_FEE_SERVICE (JSON REST API)
- COMPANION_APP (JSON REST API)

**Components using Unix timestamps**:
- PARKING_OPERATOR_ADAPTOR gRPC: `start_time_unix`, `last_updated_unix`
- UPDATE_SERVICE gRPC: `timestamp_unix`, `last_updated_unix`

**Issue #12: Mixed Timestamp Formats**

gRPC services use Unix timestamps (int64), REST APIs use ISO8601 strings.

**Impact**: Clients must handle format conversion.

**Recommendation**: This is acceptable - gRPC commonly uses Unix timestamps for efficiency. Document the convention.

---

### 2.16 Polling vs Streaming

**PARKING_APP Session Status**:
- Polls PARKING_OPERATOR_ADAPTOR every 100ms (10 updates/second)
- High polling frequency for responsive UI

**UPDATE_SERVICE Adapter State**:
- Provides `WatchAdapterStates` streaming RPC
- Event-driven, efficient

**COMPANION_APP Telemetry**:
- Polls CLOUD_GATEWAY every 10 seconds

**Issue #13: No Session Status Streaming**

PARKING_OPERATOR_ADAPTOR doesn't offer streaming for session status. High-frequency polling (10/sec) is inefficient.

**Recommendation**: Add `WatchSessionStatus` streaming RPC to PARKING_OPERATOR_ADAPTOR.

---

## 3. Data Flow Gaps

### 3.1 Parking Session Flow - Missing Integration

**Expected Flow** (from PRD):
1. Vehicle parks and locks → Session starts
2. PARKING_OPERATOR_ADAPTOR starts session with PARKING_OPERATOR
3. Session status visible in PARKING_APP

**Gap**: How does PARKING_APP know which zone to use for manual session start?

**Current Design**:
- PARKING_APP calls PARKING_FEE_SERVICE for zone lookup
- PARKING_APP passes zone_id to PARKING_OPERATOR_ADAPTOR.StartSession()

**For Automatic Lock Events**:
- PARKING_OPERATOR_ADAPTOR receives IsLocked=true signal
- PARKING_OPERATOR_ADAPTOR needs zone_id but has no way to get it

**Issue #14: Automatic Session Start Missing Zone Resolution**

The automatic lock-triggered session start flow doesn't specify how zone_id is obtained.

**Recommendation**: Either:
1. PARKING_OPERATOR_ADAPTOR calls PARKING_FEE_SERVICE directly for zone lookup, OR
2. Document that automatic sessions use a default/configured zone_id

---

### 3.2 Companion App Parking Status

**COMPANION_APP telemetry includes**:
```dart
class ParkingSession {
  final String sessionId;
  final String zoneName;
  final double hourlyRate;
  final String currency;
  final Duration duration;
  final double currentCost;
}
```

**CLOUD_GATEWAY telemetry receives** (from vehicle):
```json
{
  "parking_session_active": true
}
```

**Issue #15: COMPANION_APP Expects Detailed Session, Vehicle Only Sends Boolean**

CLOUD_GATEWAY_CLIENT only publishes whether a session is active (boolean). COMPANION_APP expects full session details (zoneName, cost, duration).

**Impact**: COMPANION_APP cannot display parking session details.

**Recommendation**: Either:
1. Extend vehicle telemetry to include session details, OR
2. COMPANION_APP should call CLOUD_GATEWAY for session details when `parking_session_active=true`

---

### 3.3 Missing COMPANION_APP → PARKING_FEE_SERVICE Integration

**COMPANION_APP communicates with**:
- CLOUD_GATEWAY (for auth, vehicle list, telemetry, commands)

**Missing**:
- Direct connection to PARKING_FEE_SERVICE for parking session details

**Issue #16: No Path for COMPANION_APP to Get Session Details**

COMPANION_APP can see `parking_session_active=true` in telemetry but cannot get session details (zone, cost, duration).

**Recommendation**:
1. Add parking session endpoint to CLOUD_GATEWAY that proxies to PARKING_FEE_SERVICE, OR
2. Document that COMPANION_APP only shows active/inactive status, not details

---

## 4. Authentication and Authorization Gaps

### 4.1 Auth Token Propagation

**COMPANION_APP** → **CLOUD_GATEWAY**: Bearer token (JWT)
**CLOUD_GATEWAY** → **CLOUD_GATEWAY_CLIENT** (MQTT): `auth_token` in message
**CLOUD_GATEWAY_CLIENT** → **LOCKING_SERVICE**: `auth_token` in gRPC request

**Issue #17: Auth Token is Passed Through Unchanged**

The same auth token is forwarded from COMPANION_APP to LOCKING_SERVICE. If COMPANION_APP uses JWT, LOCKING_SERVICE must validate JWT.

**Current Design**: LOCKING_SERVICE validates against `valid_tokens` list (demo-grade).

**Recommendation**: Document that this is demo-grade authentication only. Production would require proper token validation at each hop.

---

### 4.2 UPDATE_SERVICE Registry Authentication

**UPDATE_SERVICE** → **REGISTRY**: Bearer token (from token endpoint)

**Issue #18: Registry Credentials Not Propagated from PARKING_APP**

PARKING_APP calls `InstallAdapter` but doesn't provide registry credentials. UPDATE_SERVICE uses environment variables.

**Status**: This is acceptable for demo scope. Production would need per-user or per-tenant credentials.

---

## 5. Retry and Timeout Alignment

### 5.1 Retry Configuration

| Component | Max Retries | Base Delay | Max Delay |
|-----------|-------------|------------|-----------|
| PARKING_OPERATOR_ADAPTOR (API) | 3 | 1000ms | Not specified |
| PARKING_OPERATOR_ADAPTOR (DATA_BROKER reconnect) | 5 | 1000ms | Not specified |
| UPDATE_SERVICE (download) | 3 | 1000ms | Not specified |
| CLOUD_GATEWAY_CLIENT (MQTT reconnect) | Unlimited | 1000ms | 60000ms |
| CLOUD_GATEWAY (MQTT) | Exponential | 1s | 30s |
| PARKING_APP (zone lookup) | 3 | 1000ms | Not specified |
| PARKING_APP (DATA_BROKER reconnect) | 5 | 1000ms | Not specified |
| COMPANION_APP (API) | 3 | 1000ms | 10000ms |

**Issue #19: Inconsistent Max Delay Specifications**

Some components specify max delay, others don't. This could lead to very long delays after many retries.

**Recommendation**: Standardize max delay across all components (suggest 30 seconds).

---

### 5.2 Timeout Configuration

| Component | Operation | Timeout |
|-----------|-----------|---------|
| PARKING_OPERATOR_ADAPTOR | API call | 10000ms |
| PARKING_OPERATOR_ADAPTOR | Status poll interval | 60s |
| LOCKING_SERVICE | Execution | 500ms |
| LOCKING_SERVICE | Validation | 100ms |
| CLOUD_GATEWAY_CLIENT | Command processing | 5000ms |
| CLOUD_GATEWAY | Command timeout | 30s |
| COMPANION_APP | Connect | 10s |
| COMPANION_APP | Receive | 30s |
| COMPANION_APP | Command | 60s |

**Issue #20: Command Timeout Chain May Cascade**

COMPANION_APP waits 60s, but CLOUD_GATEWAY times out at 30s. This is correct (downstream should be shorter), but CLOUD_GATEWAY_CLIENT only waits 5s for command processing.

**Impact**: CLOUD_GATEWAY_CLIENT might time out before LOCKING_SERVICE completes if execution approaches 500ms + publish retries.

**Recommendation**: Review timeout chain to ensure:
- COMPANION_APP (60s) > CLOUD_GATEWAY (30s) > CLOUD_GATEWAY_CLIENT (5s) > LOCKING_SERVICE (500ms)

---

## 6. Summary of Critical Issues

### High Severity (Communication Breaking)

1. **Issue #4**: Telemetry nested `location` object vs flat structure - JSON parsing will fail
   - **RESOLVED**: Updated CLOUD_GATEWAY_CLIENT design to use flat structure (latitude, longitude at top level)

### Medium Severity (Functionality Gaps)

2. **Issue #6**: Adapter state enum mismatch - PARKING_APP can't represent all states
   - **RESOLVED**: Updated PARKING_APP AdapterStatus enum to align with UPDATE_SERVICE proto AdapterState
3. **Issue #10**: No zone resolution for automatic lock events
   - **RESOLVED**: Added ZoneLookupClient to PARKING_OPERATOR_ADAPTOR for automatic zone lookup during lock events
4. **Issue #15**: COMPANION_APP expects session details, vehicle only sends boolean
   - **RESOLVED**: Documented that vehicle telemetry only includes `parking_session_active` boolean; added parking session endpoint to CLOUD_GATEWAY
5. **Issue #16**: No path for COMPANION_APP to get session details
   - **RESOLVED**: Added GET /api/v1/vehicles/{vin}/parking-session endpoint to CLOUD_GATEWAY that proxies to PARKING_FEE_SERVICE

### Low Severity (Documentation/Alignment)

6. **Issue #2, #3**: Missing timestamp fields in documentation
   - **RESOLVED**: Added `timestamp` field to CLOUD_GATEWAY_CLIENT command and response message formats
7. **Issue #7**: CLOUD_GATEWAY only supports 2 door values
   - **RESOLVED**: Documented limitation in CLOUD_GATEWAY design (demo scope: only "driver" and "all" supported)
8. **Issue #11**: Inconsistent error code naming
   - **NOTED**: Acceptable for demo scope; documented convention differences
9. **Issue #13**: No session status streaming (polling inefficient)
   - **NOTED**: Acceptable for demo scope; documented as future enhancement
10. **Issue #19**: Inconsistent max delay specifications
    - **RESOLVED**: Standardized max delay to 30 seconds across all components (PARKING_APP, PARKING_OPERATOR_ADAPTOR, UPDATE_SERVICE, COMPANION_APP)

---

## 7. Recommendations Summary

### Immediate Actions (Before Implementation) - COMPLETED

1. **Align telemetry message format** - ✅ Changed to flat structure (latitude, longitude at top level)
2. **Add zone lookup to PARKING_OPERATOR_ADAPTOR** - ✅ Added ZoneLookupClient for automatic lock events
3. **Document adapter state mapping** - ✅ Updated PARKING_APP AdapterStatus enum with full mapping
4. **Add parking session details to vehicle telemetry** - ✅ Added proxy endpoint in CLOUD_GATEWAY

### Documentation Updates - COMPLETED

5. ✅ Added `timestamp` to all MQTT message format specifications
6. ✅ Documented error code conventions (acceptable differences for demo scope)
7. ✅ Standardized retry/backoff parameters (max_delay = 30s across all components)
8. ✅ Documented authentication flow limitations (demo-grade)

### Future Enhancements (Out of Scope for Current Fix)

9. Add `WatchSessionStatus` streaming RPC to PARKING_OPERATOR_ADAPTOR
10. Add dedicated MQTT topic for parking events
11. Extend CLOUD_GATEWAY door validation to all door values
12. Create shared proto definitions for common types (timestamps, error codes)

---

*Review conducted: 2026-02-04*
*Reviewer: Claude Code*
*Issues resolved: 2026-02-04*
