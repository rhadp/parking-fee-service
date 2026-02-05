# Rust Development Environment Setup

Guide for setting up a Rust development environment for RHIVOS services.

## Prerequisites

- macOS, Linux, or Windows with WSL2
- Git
- Protocol Buffer compiler (protoc)

## Install Rust

### Using rustup (Recommended)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the prompts and select the default installation.

### Verify Installation

```bash
rustc --version    # Should be 1.75+
cargo --version
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

### Build RHIVOS Services

```bash
# Build all Rust services
make build-rhivos

# Or build directly with cargo
cd rhivos && cargo build
```

### Run Tests

```bash
cd rhivos && cargo test
```

## IDE Setup

### VS Code

Install extensions:
- rust-analyzer (official Rust language server)
- Even Better TOML (for Cargo.toml editing)
- crates (dependency version hints)

Settings (`.vscode/settings.json`):
```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy"
}
```

### IntelliJ IDEA / CLion

Install the Rust plugin from JetBrains Marketplace.

## Workspace Structure

The Rust workspace is defined in `rhivos/Cargo.toml`:

```toml
[workspace]
members = [
    "shared",
    "locking-service",
    "cloud-gateway-client",
    "parking-operator-adaptor",
    "update-service"
]
```

## Common Tasks

### Generate Proto Bindings

```bash
make proto-rust
```

### Build Release Binary

```bash
cd rhivos && cargo build --release -p locking-service
```

### Run Clippy Lints

```bash
cd rhivos && cargo clippy --all-targets
```

### Format Code

```bash
cd rhivos && cargo fmt
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `protoc` not found | Install protobuf compiler |
| Build fails on proto | Run `make proto-rust` first |
| Linking errors | Install system dependencies (openssl-dev, etc.) |

## Dependencies

Key crates used:
- `tonic` - gRPC framework
- `prost` - Protocol Buffer implementation
- `tokio` - Async runtime
- `tracing` - Logging and diagnostics
