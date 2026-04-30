# Errata: Kuksa v2 API Migration (Spec 03)

## Context

The spec design.md Technology Stack references kuksa.val.v1 as the proto API
for DATA_BROKER communication. However, the project codebase uses kuksa.val.v2
exclusively. No v1 proto definition or service exists in the repository.

## Divergence

| Spec says | Implementation does |
|-----------|-------------------|
| `kuksa.val.v1` proto package | `kuksa.val.v2` proto package |
| `BrokerClient` uses v1 `Set` RPC | `BrokerClient` uses v2 `PublishValue` RPC |
| `subscribe` uses v1 API | `subscribe` uses v2 `Subscribe` RPC |
| Module path `kuksa::val::v1` | Module path `kuksa::val::v2` |
| Proto at `proto/kuksa/val.proto` | Same path, but `kuksa.val.v2` package |

## Rationale

Aligns with established codebase patterns from specs 02 (DATA_BROKER),
08 (parking-operator-adaptor), and 09 (mock-sensors), all of which use
kuksa.val.v2. See also `docs/errata/08_kuksa_v2_migration.md` for the same
decision in the parking-operator-adaptor context.

The production Kuksa Databroker (v0.5.0+) only exposes the v2 service.
Maintaining two proto versions would introduce unnecessary complexity.

## Key v2 API Differences

- **Publishing values:** Use `PublishValue(SignalID, Datapoint)` instead of v1
  `Set`. The `SignalID` uses `path` field for VSS signal paths.
- **Subscribing:** Use `Subscribe(signal_paths, buffer_size)` returning a
  `stream SubscribeResponse`. Response contains `map<string, Datapoint>`.
- **Reading values:** Use `GetValue(SignalID)` returning a `Datapoint`.
- **Value types:** Values are wrapped in `Value` message with `typed_value`
  oneof (e.g., `bool`, `float`, `string`).
