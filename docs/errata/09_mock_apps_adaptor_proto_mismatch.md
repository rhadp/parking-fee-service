# Errata: parking-app-cli Adaptor Proto Field Mismatches

**Spec:** 09_mock_apps
**Area:** parking-app-cli session override subcommands (tasks 4.3, 4.4)
**Date:** 2026-04-23

## StartSessionRequest — vehicle_id field

**Spec design.md says:**
```protobuf
message StartSessionRequest { string zone_id = 1; }
```

**Actual proto (`proto/adapter/adapter_service.proto`):**
```protobuf
message StartSessionRequest {
    string vehicle_id = 1;
    string zone_id    = 2;
}
```

**Impact:** The `parking-app-cli start-session --zone-id=<zone>` implementation
sends only `ZoneId`, leaving `VehicleId` as the zero value (empty string). This
matches the spec requirement 09-REQ-6.1 which specifies only `--zone-id` as a
CLI flag. The adaptor service is expected to resolve the vehicle identity from
its own context (e.g., the vehicle it is running on).

**Decision:** No code change. The CLI follows the spec requirement. Downstream
services that require `vehicle_id` must populate it from context, not from the
CLI mock tool. If future requirements add a `--vehicle-id` flag, the proto
already supports it.

## StopSessionRequest — session_id field

**Spec design.md says:**
```protobuf
message StopSessionRequest {}
```

**Actual proto (`proto/adapter/adapter_service.proto`):**
```protobuf
message StopSessionRequest { string session_id = 1; }
```

**Impact:** The `parking-app-cli stop-session` implementation sends an empty
`StopSessionRequest{}`, relying on the adaptor's internal session tracking to
identify the active session. This matches spec requirement 09-REQ-6.2 which
specifies no flags for `stop-session`.

**Decision:** No code change. The adaptor manages session state internally. The
empty `session_id` field is acceptable for the mock CLI use case. If explicit
session identification is needed in the future, a `--session-id` flag can be
added.
