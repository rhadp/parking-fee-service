# VSS Overlay: Standard Signals Added

## Context

The original `infra/config/kuksa/vss_overlay.json` (from task group 1) only
defined custom signals:

- `Vehicle.Command.Door.Lock`
- `Vehicle.Command.Door.LockResult`
- `Vehicle.Parking.SessionActive`

## Issue

When the Kuksa Databroker is launched with `--vss /vss_overlay.json`, the
overlay file **replaces** the default VSS model entirely. Standard VSS signals
like `Vehicle.Speed`, `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked`, and
`Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` were not available, causing
integration tests (and mock-sensors) to fail with "Path not found" errors.

## Resolution

The overlay file was updated to include all standard VSS signals used by the
system:

- `Vehicle.Speed` (float sensor)
- `Vehicle.CurrentLocation.Latitude` (double sensor)
- `Vehicle.CurrentLocation.Longitude` (double sensor)
- `Vehicle.Cabin.Door.Row1.DriverSide.IsLocked` (boolean actuator)
- `Vehicle.Cabin.Door.Row1.DriverSide.IsOpen` (boolean sensor)

This change was made during task group 8 (Integration Tests) because it was
discovered when running integration tests against a real Kuksa Databroker for
the first time.

## Impact

No design or requirements change. The design already expected these signals to
be available; the overlay file simply needed to define them since `--vss`
replaces rather than extends the default VSS model.
