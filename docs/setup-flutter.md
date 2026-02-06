# Flutter/Dart Development Environment Setup

Guide for setting up a Flutter development environment for the COMPANION_APP.

## Prerequisites

- macOS, Linux, or Windows
- Git
- Protocol Buffer compiler (protoc)

## Install Flutter

### macOS

```bash
brew install flutter
```

### Linux

```bash
# Download Flutter SDK
git clone https://github.com/flutter/flutter.git -b stable ~/flutter

# Add to PATH
echo 'export PATH="$HOME/flutter/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Windows

Download from [flutter.dev](https://flutter.dev/docs/get-started/install/windows) and add to PATH.

### Verify Installation

```bash
flutter --version    # Should be 3.x
dart --version       # Should be 3.x
flutter doctor        # Check for issues
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

### Install Dart Protoc Plugin

```bash
dart pub global activate protoc_plugin
```

Add to PATH:
```bash
export PATH="$PATH:$HOME/.pub-cache/bin"
```

## Project Setup

### Clone Repository

```bash
git clone <repository-url>
cd parking-fee-service
```

### Install Dependencies

```bash
cd android/companion_app
flutter pub get
```

### Generate Proto Bindings

```bash
make proto-dart
```

Generated code is placed in `android/companion_app/lib/generated/`.

## Project Structure

```
android/companion_app/
├── lib/
│   ├── main.dart           # App entry point
│   └── generated/          # Generated proto code
│       ├── proto.dart      # Export file
│       ├── common/         # Error types
│       ├── services/       # Service stubs
│       └── vss/            # VSS signals
├── pubspec.yaml            # Dependencies
└── analysis_options.yaml   # Lint rules
```

## Dependencies

Key dependencies in `pubspec.yaml`:

```yaml
dependencies:
  flutter:
    sdk: flutter
  grpc: ^3.2.0
  protobuf: ^3.1.0
  
dev_dependencies:
  protoc_plugin: ^21.0.0
```

## Common Tasks

### Run on Emulator/Device

```bash
cd android/companion_app
flutter run
```

### Build APK

```bash
flutter build apk
```

### Build iOS (macOS only)

```bash
flutter build ios
```

### Run Tests

```bash
flutter test
```

### Analyze Code

```bash
flutter analyze
```

## IDE Setup

### VS Code

Install extensions:
- Flutter
- Dart

Settings (`.vscode/settings.json`):
```json
{
  "dart.flutterSdkPath": "/path/to/flutter",
  "editor.formatOnSave": true
}
```

### Android Studio / IntelliJ

Install plugins:
- Flutter
- Dart

## gRPC Communication

The app communicates with backend services via gRPC:

```dart
import 'package:grpc/grpc.dart';
import 'generated/services/databroker.pbgrpc.dart';

final channel = ClientChannel(
  'rhivos-host',
  port: 50051,
  options: ChannelOptions(
    credentials: ChannelCredentials.secure(),
  ),
);

final stub = DataBrokerClient(channel);
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `flutter doctor` errors | Follow the suggested fixes |
| Proto generation fails | Install `protoc_plugin` globally |
| iOS build fails | Run `pod install` in ios/ directory |
| Hot reload not working | Restart the app |

## Platform-Specific Setup

### Android

Ensure Android SDK is installed and `ANDROID_HOME` is set.

### iOS (macOS only)

```bash
# Install Xcode from App Store
xcode-select --install

# Install CocoaPods
sudo gem install cocoapods
```

### Web

```bash
flutter config --enable-web
flutter run -d chrome
```
