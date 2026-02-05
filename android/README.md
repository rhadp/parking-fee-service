# Android Applications

Android applications for the SDV Parking Demo System.

## Directory Structure

```
android/
├── parking-app/          # Kotlin AAOS application
│   ├── app/
│   │   └── src/main/
│   │       ├── java/     # Generated protobuf/gRPC code
│   │       └── proto/    # Proto definitions (synced from root)
│   ├── build.gradle.kts  # Gradle build configuration
│   └── settings.gradle.kts
└── companion-app/        # Flutter/Dart mobile application
    ├── lib/
    │   ├── main.dart     # App entry point
    │   └── generated/    # Generated protobuf code
    └── pubspec.yaml      # Flutter dependencies
```

## Applications

### PARKING_APP (Kotlin/AAOS)

The main Android Automotive OS application that:
- Displays parking session status
- Communicates with RHIVOS services via gRPC/TLS
- Interacts with PARKING_FEE_SERVICE via HTTPS/REST

**Build:**
```bash
make build-android
# or
cd android/parking-app && ./gradlew assembleDebug
```

### COMPANION_APP (Flutter/Dart)

Mobile companion application for remote vehicle control:
- Remote lock/unlock commands
- Parking session monitoring
- Push notifications

**Build:**
```bash
cd android/companion-app && flutter build apk
```

## Dependencies

### Kotlin App
- Android SDK 33+
- Kotlin 1.9+
- gRPC-kotlin for service communication
- Protobuf-kotlin for message serialization

### Flutter App
- Flutter SDK 3.x
- Dart 3.x
- grpc package for gRPC communication
- protobuf package for message serialization

## Proto Generation

Proto bindings are generated from the root `proto/` directory:

```bash
# Generate Kotlin bindings
make proto-kotlin

# Generate Dart bindings
make proto-dart
```

Generated code is placed in:
- Kotlin: `android/parking-app/app/src/main/java/`
- Dart: `android/companion-app/lib/generated/`
