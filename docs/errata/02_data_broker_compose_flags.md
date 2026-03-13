# Erratum 02: DATA_BROKER Compose Flags and Image Tag

**Spec:** `.specs/02_data_broker/`
**Filed during:** task group 1 (write failing tests)

## Summary

The `requirements.md`, `design.md`, and `test_spec.md` for spec 02 contain
three inaccuracies about the Kuksa Databroker container image tag and CLI
flags. Implementation (task group 2) must use the corrected values documented
here.

---

## 1. Image Tag: `:0.5.1` does not exist

**Spec says:** `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.1`

**Reality:** Tag `0.5.1` is not published to `ghcr.io/eclipse-kuksa/kuksa-databroker`.
Available tags in the 0.5.x series: `0.5.0`, `0.5`.

**Correction:** Use `ghcr.io/eclipse-kuksa/kuksa-databroker:0.5.0`.

---

## 2. UDS Flag: `--uds-path` does not exist

**Spec says (design.md, test_spec.md):** `--uds-path /tmp/kuksa-databroker.sock`

**Reality:** The actual CLI flag in kuksa-databroker v0.5.0 is `--unix-socket <PATH>`.
There is no `--uds-path` flag in any published release.

**Correction:** Use `--unix-socket /tmp/kuksa-databroker.sock`.

---

## 3. VSS Loading Flag: `--metadata` does not exist

**Spec says (design.md):** `--metadata /etc/kuksa/vss-overlay.json`

**Reality:** The actual CLI flag in kuksa-databroker v0.5.0 is `--vss <FILE>`,
which accepts a comma-separated list of VSS files. The built-in standard VSS
tree is shipped as `/vss_release_4.0.json` inside the container image (not
loaded automatically when `--vss` is supplied).

To load both the standard VSS 4.0 tree **and** the custom overlay:

```
--vss /vss_release_4.0.json,/etc/kuksa/vss-overlay.json
```

**Correction:** Replace `--metadata` with `--vss` and include the full VSS
comma-separated path.

---

## 4. Subscribe RPC Field Name: `signal_paths` does not exist

**Spec says (test_spec.md):** `grpc_subscribe("Vehicle.Parking.SessionActive")`
with implied field `signal_paths`.

**Reality:** The `kuksa.val.v1.SubscribeRequest` message has a field named
`entries` (not `signal_paths`), each entry being a `SubscribeEntry` with
`path`, `view`, and `fields`.

Correct grpcurl body:
```json
{"entries": [{"path": "Vehicle.Parking.SessionActive", "fields": ["FIELD_VALUE"]}]}
```

---

## 5. GetMetadata RPC does not exist

**Spec says (test_spec.md):** `grpc_get_metadata("Vehicle.Speed")` via a
dedicated `GetMetadata` RPC.

**Reality:** `kuksa.val.v1.VAL` has no `GetMetadata` method. Metadata is
retrieved via the `Get` RPC with `fields: ["FIELD_METADATA"]`.

Correct grpcurl body:
```json
{"entries": [{"path": "Vehicle.Speed", "fields": ["FIELD_METADATA"]}]}
```

---

## 6. Set RPC Requires `fields` in EntryUpdate

**Spec says (test_spec.md):** `grpc_set("Vehicle.Parking.SessionActive", true)`
with an implied simple value body.

**Reality:** `kuksa.val.v1.SetRequest.updates` is a list of `EntryUpdate`
messages that each require a `fields` list specifying which fields to write.
Without `fields: ["FIELD_VALUE"]`, the set operation is a no-op.

Correct grpcurl body:
```json
{"updates": [{"entry": {"path": "...", "value": {"bool": true}}, "fields": ["FIELD_VALUE"]}]}
```

---

## Impact on Tests

The integration tests in `tests/databroker/` use the corrected values above.
The `compose_test.go` tests check for `--unix-socket` (not `--uds-path`) and
`kuksa-databroker:0.5.0` (not `:0.5.1`). Inline comments in each test
reference the relevant spec test ID and note the divergence.
