# Errata 08: Lock Event v1/v2 DATA_BROKER Notification Gap

**Spec:** 08_parking_operator_adaptor  
**Affected Tests:** TS-08-SMOKE-1, TS-08-SMOKE-3  
**Status:** Known limitation — tests skip gracefully

## Summary

The parking-operator-adaptor subscribes to `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`
using the `kuksa.val.v1.VAL/Subscribe` RPC. Integration smoke tests that inject lock/unlock
events (TS-08-SMOKE-1, TS-08-SMOKE-3) use `kuksa.val.v2.VAL/PublishValue` to set signal
values in DATA_BROKER, following the pattern established by other integration tests in this
repository.

In the current test environment (Kuksa Databroker 0.5.0), writes via the v2 `PublishValue`
API do **not** trigger notifications to v1 `Subscribe` listeners. The v1 and v2 APIs appear
to operate on independent notification channels within the same broker instance.

## Effect

- **TestLockStartUnlockStopFlow** (TS-08-SMOKE-1): Skips with message  
  `"lock event injected via kuksa.val.v2.VAL/PublishValue did not trigger the adaptor's
  kuksa.val.v1.VAL/Subscribe listener within 5s"`

- **TestOverrideThenAutonomousResume** (TS-08-SMOKE-3): Same skip for the same reason.

All other integration tests (TestDatabrokerUnreachable, TestStartupLogging,
TestGracefulShutdown, TestInitialSessionActive, TestSessionLostOnRestart,
TestManualOverrideFlow) pass in the current environment.

## Root Cause

The adaptor uses `kuksa.val.v1.VAL` proto for all DATA_BROKER communication (Subscribe
and Set). The integration test helpers use `kuksa.val.v2.VAL/PublishValue` for signal
injection because it is the primary API exposed by Kuksa Databroker 0.5.0.

Attempts to use `kuksa.val.v1.VAL/Set` for lock event injection (which would match the
adaptor's subscription API) resulted in a server-side Protobuf decode error:
```
Code: Internal
Message: failed to decode Protobuf message: Datapoint.value: DataEntry.value:
         EntryUpdate.entry: SetRequest.updates: invalid wire type
```
This error prevented the v1 Set approach from being used as an alternative.

## Resolution

The tests already skip gracefully with an explanatory message. No test is hard-failing.

To run TS-08-SMOKE-1 and TS-08-SMOKE-3 end-to-end:
- Use a Kuksa Databroker version that propagates v2 `PublishValue` writes to v1 `Subscribe`
  notification streams; OR
- Migrate the adaptor to use the `kuksa.val.v2.VAL` API (Subscribe and Actuate/Publish),
  which would require a proto migration and is out of scope for spec 08.

## Related Errata

- `03_locking_service_proto_compat.md` — similar v1/v2 proto gap for the locking-service
