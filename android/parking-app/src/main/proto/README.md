# Protocol Buffer Definitions

Shared Protocol Buffer definitions for all inter-component communication in the SDV Parking Demo System.

## Directory Structure

```
proto/
├── buf.yaml              # Buf CLI configuration
├── buf.gen.yaml          # Code generation configuration
├── common/               # Shared message types
│   └── error.proto       # Error handling types
├── services/             # Service interface definitions
│   ├── databroker.proto  # DATA_BROKER service (Kuksa-compatible)
│   ├── locking_service.proto    # LOCKING_SERVICE interface
│   ├── parking_adaptor.proto    # PARKING_OPERATOR_ADAPTOR interface
│   └── update_service.proto     # UPDATE_SERVICE interface
└── vss/                  # Vehicle Signal Specification types
    └── signals.proto     # VSS signal message definitions
```

## Services

| Proto File | Service | Description |
|------------|---------|-------------|
| `databroker.proto` | DataBroker | VSS signal get/set/subscribe operations |
| `locking_service.proto` | LockingService | Door lock/unlock commands |
| `parking_adaptor.proto` | ParkingAdaptor | Parking session management |
| `update_service.proto` | UpdateService | Adapter lifecycle management |

## VSS Signals

Defined in `vss/signals.proto`:

| Signal | Type | Description |
|--------|------|-------------|
| DoorState | message | Lock and open status for doors |
| Location | message | Vehicle GPS coordinates |
| VehicleSpeed | message | Current vehicle speed |
| ParkingState | message | Active parking session info |

## Code Generation

Generate language bindings using:

```bash
# Generate all language bindings
make proto

# Generate specific language
make proto-rust    # Rust bindings
make proto-kotlin  # Kotlin bindings
make proto-dart    # Dart bindings
make proto-go      # Go bindings
```

## Output Locations

| Language | Output Directory |
|----------|------------------|
| Rust | `rhivos/shared/src/proto/` |
| Kotlin | `android/parking-app/app/src/main/java/` |
| Dart | `android/companion_app/lib/generated/` |
| Go | `backend/gen/` |

## Linting

Proto files are linted using buf:

```bash
cd proto && buf lint
```

## Dependencies

- buf CLI (recommended) or protoc
- Language-specific protoc plugins

## Adding New Definitions

1. Create or modify `.proto` files in appropriate directory
2. Run `buf lint` to validate
3. Run `make proto` to regenerate all bindings
4. Commit both proto files and generated code
