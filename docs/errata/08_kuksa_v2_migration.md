# Errata: Kuksa v2 API Migration (Spec 08)

## Context

The spec design.md Technology Stack references kuksa.val.v1 as the proto API
for DATA_BROKER communication. However, the project codebase uses kuksa.val.v2
exclusively. No v1 proto definition or service exists in the repository.

## Divergence

| Spec says | Implementation does |
|-----------|-------------------|
| `kuksa.val.v1` proto package | `kuksa.val.v2` proto package |
| `BrokerClient` uses v1 `Set` RPC | `BrokerClient` uses v2 `PublishValue` RPC |
| `subscribe_bool` uses v1 API | `subscribe` uses v2 `Subscribe` RPC |
| Module path `kuksa::val::v1` | Module path `kuksa::val::v2` |

## Rationale

Aligns with established codebase patterns from specs 02 (DATA_BROKER),
03 (locking-service), 04 (cloud-gateway-client), and 09 (mock-sensors),
all of which use kuksa.val.v2. See also
`docs/errata/09_mock_apps_sensor_proto_compat.md` for the same decision
in the mock-sensors context.

The production Kuksa Databroker (v0.5.0+) only exposes the v2 service.
Maintaining two proto versions would introduce unnecessary complexity.

## Key v2 API Differences

- **Publishing values:** Use `PublishValue(SignalID, Datapoint)` instead of v1
  `Set`. The `SignalID` uses `path` field for VSS signal paths.
- **Subscribing:** Use `Subscribe(signal_paths, buffer_size)` returning a
  `stream SubscribeResponse`. Response contains `map<string, Datapoint>`.
- **Value types:** Values are wrapped in `Value` message with `typed_value`
  oneof (e.g., `bool`, `int64`, `string`).

## DATA_BROKER Retry Clarification

Spec 08-REQ-3.E3 specifies "retry with exponential backoff (1s, 2s, 4s) up to
5 attempts". This is contradictory: three delay values cover 4 attempts (1
initial + 3 retries). The implementation uses 5 total attempts with delays
1s, 2s, 4s, 8s (4 delays between 5 attempts), consistent with tasks.md section
4.1.
