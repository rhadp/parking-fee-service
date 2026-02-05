# Core Requirement: Protocol Buffers Compiler (protoc)

Install via Homebrew (recommended):

```bash
brew install protobuf
```

Verify installation:

```bash
protoc --version
```

## Language-Specific Setup

### Go

Install the Go protobuf and gRPC plugins:

```bash
go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest
```

Add to your shell profile (`.zshrc`):

```bash
export PATH="$PATH:$(go env GOPATH)/bin"
```

### Rust

Rust uses build-time code generation, so no global install is needed beyond `protoc`. Add these to your `Cargo.toml`:

```toml
[dependencies]
tonic = "0.12"
prost = "0.13"
tokio = { version = "1", features = ["full"] }

[build-dependencies]
tonic-build = "0.12"
```

The `tonic-build` crate invokes `protoc` during `cargo build` via a `build.rs` script.

### Kotlin

Kotlin/Gradle handles protobuf compilation via plugins—no global tools needed beyond `protoc`. In your `build.gradle.kts`:

```kotlin
plugins {
    id("com.google.protobuf") version "0.9.4"
}

dependencies {
    implementation("io.grpc:grpc-kotlin-stub:1.4.1")
    implementation("io.grpc:grpc-protobuf:1.62.2")
    implementation("com.google.protobuf:protobuf-kotlin:3.25.3")
}

protobuf {
    protoc {
        artifact = "com.google.protobuf:protoc:3.25.3"
    }
    plugins {
        create("grpc") {
            artifact = "io.grpc:protoc-gen-grpc-java:1.62.2"
        }
        create("grpckt") {
            artifact = "io.grpc:protoc-gen-grpc-kotlin:1.4.1:jdk8@jar"
        }
    }
}
```

## Summary: What to Install Globally

| Tool | Install Command |
|------|----------------|
| Protocol Buffers | `brew install protobuf` |
| Go protoc plugins | `go install google.golang.org/protobuf/cmd/protoc-gen-go@latest && go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest` |
| Rust | Nothing extra (handled by Cargo) |
| Kotlin | Nothing extra (handled by Gradle) |

## Optional: Buf CLI

For a more modern protobuf workflow with linting, breaking change detection, and cross-language code generation:

```bash
brew install bufbuild/buf/buf
```

Buf can simplify managing `.proto` files across your Go, Rust, and Kotlin services.