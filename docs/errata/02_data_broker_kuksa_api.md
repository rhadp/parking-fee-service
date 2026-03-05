# Errata: DATA_BROKER Kuksa 0.5.0 API Divergences

## Context

During implementation of task group 3 (VSS overlay), several differences
between the design document assumptions and actual Kuksa Databroker 0.5.0
behavior were discovered.

## Divergences

### 1. No `--vss-overlay` CLI flag

**Design says:** `--vss config/vss/vss.json --vss-overlay config/vss/vss_overlay.json`

**Actual behavior:** Kuksa 0.5.0 has only `--vss <FILE>` which accepts a
comma-separated list of files. Overlays are loaded by passing multiple files:
`--vss /config/vss_v5.1.json,/config/vss_overlay.json`

### 2. VSS file compatibility

**Design says:** Load standard VSS v5.1 JSON file.

**Actual behavior:** Kuksa 0.5.0 ships with `vss_release_4.0.json` internally.
The COVESA VSS v5.1 JSON uses `float` values for `min`/`max` fields (e.g.
`"max": 0.0`) which Kuksa 0.5.0 rejects (`expected i16`). The VSS v5.1 JSON
was patched to convert whole-number floats to integers for compatibility.

### 3. `PublishValue` instead of `Actuate`

**Design says:** Use `SetValue` / `SetRequest` for writing signal values.

**Actual behavior:** Kuksa 0.5.0 v2 API has `Actuate` and `PublishValue`.
`Actuate` requires a registered provider and fails with `UNAVAILABLE` if none
exists. `PublishValue` writes values directly and is the correct RPC for
test harnesses and services that publish sensor/actuator values.

### 4. `ListMetadata` does not populate Path field

**Design says:** Query signal metadata including path.

**Actual behavior:** When `ListMetadata` is called with a specific signal as
`root` (e.g. `Vehicle.Speed`), it returns exactly one metadata entry but the
`path` field in the response is empty. The signal identity is determined by the
query root parameter, not the response path field.

### 5. Overlay JSON requires `description` on all branches

**Design says:** Overlay JSON format per COVESA spec.

**Actual behavior:** Kuksa 0.5.0 requires a `description` field on every
branch node in the overlay JSON. Missing descriptions cause a parse error.
