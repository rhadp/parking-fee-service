# SDV Parking Companion App

Flutter/Dart mobile companion application for the SDV Parking Demo System.

## Overview

This companion app provides a mobile interface for users to:
- View parking session status
- Manage parking payments
- Receive notifications about parking events
- Communicate with backend services via gRPC

## Prerequisites

- Flutter SDK >= 3.16.0
- Dart SDK >= 3.2.0
- Protocol Buffers compiler (`protoc`)
- Dart protoc plugin

## Setup

1. Install Flutter dependencies:
   ```bash
   flutter pub get
   ```

2. Generate protobuf code (from project root):
   ```bash
   make proto-dart
   ```

   Or manually:
   ```bash
   dart run build_runner build
   ```

## Project Structure

```
companion-app/
├── lib/
│   ├── main.dart           # Application entry point
│   ├── generated/          # Generated protobuf/gRPC code
│   ├── services/           # gRPC service clients
│   ├── models/             # Data models
│   ├── screens/            # UI screens
│   └── widgets/            # Reusable widgets
├── test/                   # Unit and widget tests
├── pubspec.yaml            # Flutter dependencies
└── analysis_options.yaml   # Dart analyzer configuration
```

## Dependencies

### gRPC & Protocol Buffers
- `grpc`: gRPC client for Dart
- `protobuf`: Protocol Buffers runtime
- `protoc_plugin`: Code generation for proto files

### State Management
- `provider`: State management solution

### Networking
- `http`: HTTP client for REST APIs
- `dio`: Advanced HTTP client

### Location Services
- `geolocator`: GPS location access
- `permission_handler`: Runtime permissions

## Communication

The app communicates with:
- **PARKING_FEE_SERVICE**: REST/HTTPS for parking operations
- **Backend gRPC services**: For real-time updates

## Development

### Running the app
```bash
flutter run
```

### Running tests
```bash
flutter test
```

### Building for release
```bash
flutter build apk --release
```

## Related Documentation

- [Flutter Setup Guide](../../docs/setup-flutter.md)
- [Protocol Buffer Definitions](../../proto/README.md)
- [Communication Architecture](../../docs/local-infrastructure.md)
