# Android/Kotlin Development Environment Setup

Guide for setting up an Android development environment for the PARKING_APP.

## Prerequisites

- macOS, Linux, or Windows
- Git
- JDK 17+
- Protocol Buffer compiler (protoc)

## Install Android Studio

1. Download Android Studio from [developer.android.com](https://developer.android.com/studio)
2. Run the installer and follow the setup wizard
3. Install Android SDK 33+ via SDK Manager

### Verify Installation

```bash
# Check Java version
java -version    # Should be 17+

# Check Android SDK (after setup)
$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager --version
```

## Install Protocol Buffer Compiler

### macOS

```bash
brew install protobuf
```

### Linux (Debian/Ubuntu)

```bash
sudo apt-get install -y protobuf-compiler
```

### Verify

```bash
protoc --version   # Should be 3.x or higher
```

## Project Setup

### Clone Repository

```bash
git clone <repository-url>
cd parking-fee-service
```

### Open in Android Studio

1. Open Android Studio
2. Select "Open" and navigate to `android/parking-app`
3. Wait for Gradle sync to complete

### Build from Command Line

```bash
# Build debug APK
make build-android

# Or use Gradle directly
cd android/parking-app && ./gradlew assembleDebug
```

## Generate Proto Bindings

```bash
make proto-kotlin
```

Generated code is placed in `android/parking-app/app/src/main/java/`.

## Project Structure

```
android/parking-app/
├── app/
│   └── src/main/
│       ├── java/           # Kotlin/Java source + generated proto
│       ├── res/            # Android resources
│       └── AndroidManifest.xml
├── build.gradle.kts        # App build configuration
└── settings.gradle.kts     # Project settings
```

## Dependencies

Key dependencies in `build.gradle.kts`:

```kotlin
dependencies {
    // gRPC
    implementation("io.grpc:grpc-kotlin-stub:1.4.0")
    implementation("io.grpc:grpc-okhttp:1.58.0")
    
    // Protocol Buffers
    implementation("com.google.protobuf:protobuf-kotlin:3.24.0")
}
```

## Common Tasks

### Run on Emulator

1. Create an Android Automotive emulator in AVD Manager
2. Select "Automotive" system image with Play Store
3. Run the app from Android Studio

### Run on Device

1. Enable Developer Options on device
2. Enable USB Debugging
3. Connect device via USB
4. Run from Android Studio

### Run Tests

```bash
cd android/parking-app && ./gradlew test
```

## IDE Configuration

### Code Style

Import the project code style:
1. Preferences → Editor → Code Style → Kotlin
2. Set to "Kotlin style guide"

### Lint Configuration

Enable Android Lint checks in `build.gradle.kts`:

```kotlin
android {
    lint {
        warningsAsErrors = true
        abortOnError = true
    }
}
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Gradle sync fails | Check JDK version (17+) |
| Proto not found | Run `make proto-kotlin` |
| SDK not found | Set `ANDROID_HOME` environment variable |
| Emulator won't start | Enable hardware acceleration (HAXM/KVM) |

## gRPC Communication

The app communicates with RHIVOS services via gRPC/TLS:

```kotlin
val channel = ManagedChannelBuilder
    .forAddress("rhivos-host", 50051)
    .useTransportSecurity()
    .build()

val stub = DataBrokerGrpcKt.DataBrokerCoroutineStub(channel)
```

See `infra/config/endpoints.yaml` for service addresses.
