# PARKING_APP Implementation Divergences from Design

## Project Location

- **Design doc:** `aaos/parking-app/`
- **Actual:** `android/parking-app/`
- **Reason:** The repository already contained an `android/parking-app/.gitkeep`
  placeholder. Using the existing directory avoids creating a parallel structure.

## AGP and Gradle Versions

- **Design doc:** Android Gradle Plugin 8.x, Gradle 8.x
- **Actual:** AGP 9.0.1, Gradle 9.1, Kotlin 2.2.10
- **Reason:** The only available JDK on the build machine is Java 25, which
  requires Gradle 9.1+. AGP 9.0.1 is the matching stable Android plugin.
  AGP 9.0 provides built-in Kotlin support (Kotlin 2.2.10), so the
  `org.jetbrains.kotlin.android` plugin is not explicitly applied.

## protobuf-gradle-plugin and `android.newDsl=false`

- **Design doc:** Standard protobuf-gradle-plugin usage
- **Actual:** `android.newDsl=false` set in `gradle.properties`
- **Reason:** protobuf-gradle-plugin 0.9.6 relies on `BaseExtension` which
  was replaced in AGP 9.0's new DSL mode. The `newDsl=false` flag restores
  the old API. This is a temporary workaround until the plugin ships AGP 9
  support. The flag will be removed in AGP 10.0.

## No Kotlin Protobuf Extensions

- **Design doc:** Uses `protobuf-kotlin-lite` with Kotlin builtins
- **Actual:** Uses `protobuf-javalite` only; Kotlin builtin codegen disabled
- **Reason:** The vendored Kuksa protos use package `kuksa.val.v2` where `val`
  is a Kotlin keyword. The Kotlin protobuf codegen generates code with
  unescaped package references (e.g. `kuksa.val.v2.Types.Datapoint`), causing
  compilation errors. Java-lite classes are used directly from Kotlin instead.
  The gRPC Kotlin stubs (`grpckt`) are still generated and work correctly.

## Protobuf and gRPC Versions

- **Design doc:** protobuf 3.25.3, grpc 1.62.2
- **Actual:** protobuf 4.29.4, grpc 1.68.2
- **Reason:** protobuf 3.25.x generates Kotlin code incompatible with
  Kotlin 2.2 (DslMap type parameter issues). Upgraded to protobuf 4.29.4
  and matching grpc 1.68.2 for compatibility.

## Proto Source Configuration

- **Design doc:** `sourceSets { main { proto { srcDir("../../proto") } } }`
- **Actual:** Symlinks from `app/src/main/proto/` to repo root `proto/`
- **Reason:** The protobuf-gradle-plugin's `proto` source set accessor is not
  available in Kotlin DSL with AGP 9.0. Symlinks provide equivalent behavior
  while keeping proto files in the default location the plugin expects.
