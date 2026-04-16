# Errata: DATA_BROKER Compose Configuration and API Flags

**Related Spec:** 02_data_broker
**Date:** 2026-04-17

## Summary

Three specification requirements for the DATA_BROKER are unachievable as written
because they reference CLI flags and container image tags that do not exist in
the actual `ghcr.io/eclipse-kuksa/kuksa-databroker` binary at any released
version.  This errata documents the actual behavior and the mitigations applied
in the implementation.

---

## Errata 1: Image Tag :0.5.1 Does Not Exist

**Affected requirements:** 02-REQ-1.1, 02-REQ-1.2 and their acceptance criteria.

**Spec text:** The spec mandates
`ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.1` as the pinned image.

**Reality:** Tag `:0.5.1` does not exist in the `ghcr.io/eclipse-kuksa/kuksa-databroker`
registry.  The latest available release at the time of implementation is `:0.5.0`.

**Mitigation:** Use `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0` in
`deployments/compose.yml`.  Integration tests check for the `:0.5.0` suffix
rather than `:0.5.1`.

---

## Errata 2: --uds-path Flag Does Not Exist

**Affected requirements:** 02-REQ-3.1 and its acceptance criterion 1.

**Spec text:** Acceptance criterion 1 of 02-REQ-3 states:
> The DATA_BROKER command args SHALL include `--uds-path /tmp/kuksa-databroker.sock`.

**Reality:** The `kuksa-databroker` binary (0.5.0) does not have a `--uds-path`
flag.  The correct flag is `--unix-socket`.

**Mitigation:** Use `--unix-socket /tmp/kuksa-databroker.sock` in the compose
command args.  Tests check for `--unix-socket` rather than `--uds-path`.

---

## Errata 3: --address Flag Does Not Accept host:port Combined Format

**Affected requirements:** 02-REQ-2.1 and its acceptance criterion 3.

**Spec text:** Acceptance criterion 3 of 02-REQ-2 states:
> The DATA_BROKER command args SHALL include `--address 0.0.0.0:55555`.

**Reality:** The `kuksa-databroker` binary requires `--address` and `--port` as
separate flags.  `--address 0.0.0.0:55555` is silently ignored or causes an
error; the correct invocation is `--address 0.0.0.0 --port 55555`.

**Mitigation:** Use `--address 0.0.0.0` + `--port 55555` in the compose command
args.  Tests check that both `0.0.0.0` and `55555` appear in the command args.

---

## Errata 4: No GetMetadata RPC in kuksa.val.v2

**Affected test specs:** TS-02-1, TS-02-2, TS-02-4, TS-02-5, TS-02-P1, TS-02-SMOKE-2.

**Spec text:** Test pseudocode uses `conn.GetMetadata(signal)` throughout.

**Reality:** The `kuksa.val.v2.VAL` service does not have a `GetMetadata` RPC.
The correct RPC for metadata queries is `ListMetadata`.  Signal paths are not
included in the `ListMetadata` response body, so they must be prepended from
the request when matching output.

**Mitigation:** Tests use `kuksa.val.v2.VAL/ListMetadata` with a `root` filter
and prepend the signal path to the response for matching.

---

## Errata 5: gRPC API Version is kuksa.val.v2

**Affected test specs:** All live gRPC tests.

**Spec text:** The design references "gRPC v2 API" but test pseudocode does not
specify a package name.

**Reality:** `kuksa-databroker:0.5.0` exposes `kuksa.val.v2.VAL` (not
`kuksa.val.v1.VAL`).  Key methods used by tests:
- `kuksa.val.v2.VAL/GetServerInfo` — health check
- `kuksa.val.v2.VAL/ListMetadata` — signal metadata
- `kuksa.val.v2.VAL/GetValue` — read signal value
- `kuksa.val.v2.VAL/PublishValue` — write signal value (sensors and actuators)
- `kuksa.val.v2.VAL/Subscribe` — stream value changes (`signal_ids` field)

---

## Errata 6: VSS Version is 4.0 (Not 5.1)

**Affected requirements:** 02-REQ-5 and its sub-requirements.

**Spec text:** Signals are described as "standard VSS v5.1 signals."

**Reality:** `kuksa-databroker:0.5.0` bundles `/vss_release_4.0.json` (VSS 4.0)
as its default signal tree.  The `--vss` flag in compose.yml loads this file
together with the custom overlay.  All 5 standard signals listed in 02-REQ-5.1
are present in VSS 4.0, so the functional requirements are met.

**Mitigation:** Tests check signal availability by path and type, not by VSS
version string.  The version claim in the spec is an editorial error.

---

## Impact on Tests

| Errata | Test impact |
|--------|-------------|
| 1 (image tag) | Tests check for `:0.5.0` not `:0.5.1` |
| 2 (--uds-path) | Tests check for `--unix-socket` not `--uds-path` |
| 3 (--address format) | Tests check `0.0.0.0` and `55555` separately |
| 4 (GetMetadata) | Tests use `ListMetadata` with path prepend |
| 5 (API version) | Tests use `kuksa.val.v2.VAL/` prefix for all RPCs |
| 6 (VSS version) | Tests check signal paths/types, not version |
