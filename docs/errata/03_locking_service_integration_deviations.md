# Erratum 03: LOCKING_SERVICE Integration Test Deviations

**Spec:** `.specs/03_locking_service/`
**Filed during:** task group 6 (integration test validation)

## Summary

Three deviations from the spec assumptions were discovered during integration
testing of the LOCKING_SERVICE against a live Kuksa Databroker v0.5.0.

---

## 1. Compose `--address` flag takes IP only (not IP:PORT)

**Spec says (implicitly):** deployments/compose.yml configures the databroker
bind address.

**Reality:** The `--address` flag for kuksa-databroker v0.5.0 accepts an IP
address only, not `IP:PORT`. The previous compose.yml value `0.0.0.0:55555`
caused `Error: AddrParseError(Ip)` and the container exited immediately.

**Correction applied:**
```yaml
# Before (broken):
- "--address"
- "0.0.0.0:55555"

# After (correct):
- "--address"
- "0.0.0.0"
```

---

## 2. `--vss` flag must include the standard VSS tree

**Spec says (design.md):** The overlay file provides all necessary signal
metadata.

**Reality:** The custom overlay (`vss-overlay.json`) only defines the three
project-specific signals (`Vehicle.Parking.SessionActive`,
`Vehicle.Command.Door.Lock`, `Vehicle.Command.Door.Response`). Standard
VSS signals used by the locking-service (`Vehicle.Speed`,
`Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`, etc.) must be loaded from
the built-in `/vss_release_4.0.json` file shipped in the container image.

**Correction applied:**
```yaml
# Before:
- "--vss"
- "/etc/kuksa/vss-overlay.json"

# After:
- "--vss"
- "/vss_release_4.0.json,/etc/kuksa/vss-overlay.json"
```

---

## 3. `kuksa.val.v1.VAL` is not registered with gRPC reflection in v0.5.0

**Spec says:** The LOCKING_SERVICE uses the `kuksa.val.v1` gRPC service.

**Reality:** Kuksa Databroker v0.5.0 exposes BOTH `kuksa.val.v1.VAL` (legacy)
and `kuksa.val.v2.VAL`, but only `kuksa.val.v2.VAL` is registered with gRPC
server reflection. The v1 API is fully functional when called with compiled
proto definitions (as the Rust `tonic`-based locking-service does), but
`grpcurl` — which relies on reflection for service discovery — reports v1 as
unavailable.

**Impact on integration tests:** All `grpcurl` invocations in the Go
integration test helpers must pass explicit proto file flags:
```
grpcurl -import-path {protoDir} -proto kuksa/val/v1/val.proto ...
```

The health check uses the v2 `GetServerInfo` (which IS in reflection) rather
than v1.

**No code change needed in the locking-service** — the Rust binary uses
compiled proto bindings and communicates with v1 API correctly.

---

## 4. Integration tests use `podman run` (not compose) for isolation

**Spec says (test_spec.md):** Integration tests use shared helpers for
starting/stopping DATA_BROKER.

**Reality:** The integration test helpers start the Kuksa Databroker with a
direct `podman run` command (naming the container `ls-test-databroker`) rather
than `podman compose`. This avoids the macOS Podman VM bind-mount limitation
that prevents the UDS volume (`kuksa-uds`) from being mounted from host paths
under `/tmp`. The locking-service integration tests only require TCP access to
the databroker — UDS connectivity is not tested here (it is covered by
`tests/databroker/` tests).
