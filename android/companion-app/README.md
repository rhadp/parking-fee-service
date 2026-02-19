# COMPANION_APP

Flutter/Dart mobile application for remote vehicle control via the
CLOUD_GATEWAY REST API (specification 07).

## Features

- Vehicle pairing via VIN + PIN
- Real-time vehicle status display (lock state, door, speed, location, parking)
- Remote lock/unlock commands with result feedback
- Token persistence across app restarts

## Build

```bash
# Get dependencies
flutter pub get

# Run analyzer
flutter analyze

# Run tests
flutter test

# Build debug APK
flutter build apk --debug
```

## Configuration

The default CLOUD_GATEWAY base URL is `http://10.0.2.2:8081` (Android emulator
alias for host localhost). This can be configured at runtime via the settings
screen.

## Project Structure

```
lib/
  main.dart                    # App entry point, Provider setup
  models/models.dart           # Data classes (VehicleStatus, PairResponse, etc.)
  services/cloud_gateway_client.dart  # REST client for CLOUD_GATEWAY
  providers/vehicle_provider.dart     # State management (ChangeNotifier)
  screens/pairing_screen.dart         # VIN + PIN pairing screen
  screens/dashboard_screen.dart       # Vehicle status + lock/unlock
```
