# Errata: Mock Sensors Proto Compatibility (Spec 09)

## Context

The mock sensors (`location-sensor`, `speed-sensor`, `door-sensor`) target the
**kuksa.val.v1** gRPC API (`kuksa.val.v1.VAL.Set`). The production
kuksa-databroker (v0.5.0+) exposes only **kuksa.val.v2** (`kuksa.val.v2.VAL`).

## Impact

Integration tests that run mock sensors against a live kuksa-databroker will
fail because the v1 `VAL` service is not registered by the production broker.

## Mitigation

1. **Integration tests skip** when the target DATA_BROKER does not expose the
   `kuksa.val.v1.VAL` service. Tests use a gRPC health/reflection check or
   catch the `UNIMPLEMENTED` status code and call `t.Skip(...)`.

2. **Automated testing** uses a **stub v1-compatible gRPC server** that
   implements the `kuksa.val.v1.VAL.Set` RPC and stores values in memory.
   This stub is started in `TestMain` or per-test and provides a deterministic
   environment for verifying sensor publish behavior.

3. **Future alignment**: when the project upgrades to kuksa.val.v2, the mock
   sensors and their proto vendoring will be updated accordingly.

## Spec Divergence

- **design.md** states: *"The mock sensors target the kuksa.val.v1 gRPC API."*
- **requirements.md** (09-REQ-10.1) requires vendored kuksa.val.v1 protos.
- This errata documents the known incompatibility with the production broker
  and the testing strategy to work around it.
