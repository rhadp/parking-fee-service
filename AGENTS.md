# Agent Instructions

This file provides instructions for AI coding agents working on this repository.

## About the Repo

Parking Fee Service - a Software-Defined Vehicle (SDV) demo system showcasing mixed-criticality architecture.
Demonstrates automatic parking fee payment through vehicle-to-cloud integration.

See [prd.md](prd.md) for full product requirements.

## Spec-Driven Development

### Requirements

This project uses "Spec-Driven Development" as its primary means to create a working implementation.

The guiding documents that provide the requirements, design and list of tasks to be done live in `.kiro/specs/{feature-name}/` with:

- `requirements.md` - User stories and acceptance criteria
- `design.md` - Architecture and interface definitions
- `tasks.md` - Implementation task list

A guideline on how to write specifications is in [requirements-engineering.md](.kiro/steering/requirements-engineering.md)

For Claude: @.kiro/steering/requirements-engineering.md

### Architecture Decision Records

An architecture decision record (ADR) is a document that captures an important architecture decision made
along with its context and consequences. ADRs live in `docs/adr/{decision.md}`.

### Other Documentation

Other misc. documentation markdown files live in `docs/{topic.md}`

## Project Structure

```
parking-fee-service/
├── rhivos/                         # Rust services (RHIVOS)
│   ├── locking-service/            # ASIL-B door locking (safety partition)
│   ├── data-broker/                # Kuksa integration wrapper
│   ├── cloud-gateway-client/       # MQTT client (safety partition)
│   ├── parking-operator-adaptor/   # Dynamic adapter (QM partition)
│   ├── update-service/             # Container lifecycle (QM partition)
│   └── shared/                     # Shared Rust libraries
├── android/
│   ├── parking-app/                # Kotlin AAOS application
│   └── companion-app/              # Flutter/Dart mobile app
├── backend/
│   ├── parking-fee-service/        # Go parking operations service
│   └── cloud-gateway/              # Go MQTT broker/router
├── proto/                          # Shared Protocol Buffer definitions
│   ├── vss/                        # VSS signal definitions
│   ├── services/                   # Service interface definitions
│   └── common/                     # Shared message types
├── containers/                     # Containerfiles
│   ├── rhivos/                     # RHIVOS service containers
│   ├── backend/                    # Backend service containers
│   └── mock/                       # Mock service containers
├── infra/                          # Local development infrastructure
│   ├── compose/                    # Podman compose files
│   ├── certs/                      # Development TLS certificates
│   └── config/                     # Service configurations
├── scripts/                        # Build and utility scripts
├── docs/                           # Documentation
└── Makefile                        # Root build orchestration
```

## Development Guidelines

### Quick Reference: Commands by Language

| Language | Location | Build | Test | Lint | Format |
|----------|----------|-------|------|------|--------|
| **Go** | `backend/` | `go build ./...` | `go test -short ./...` | `golangci-lint run ./...` | `gofmt -w .` |
| **Rust** | `rhivos/` | `cargo build --workspace` | `cargo test --workspace` | `cargo clippy --all-targets` | `cargo fmt` |
| **Kotlin** | `android/parking-app/` | `./gradlew assembleDebug` | `./gradlew test` | `./gradlew lint` | (Android Studio) |
| **Flutter/Dart** | `android/companion-app/` | `flutter build apk` | `flutter test` | `flutter analyze` | `dart format .` |

Use `make test` to run all tests, or `make test-rhivos`, `make test-android`, `make test-backend` for specific stacks.

### Code Standards by Language

#### Go (Backend Services)

- **Version**: Go 1.24+
- **Location**: `backend/`
- **Testing**: `cd backend && go test -short ./...` (full tests run in CI)
- **Linting**: `cd backend && golangci-lint run ./...` (baseline warnings documented in [docs/LINTING.md](docs/LINTING.md))
- **Formatting**: `gofmt -w .`
- **Setup guide**: [docs/setup-go.md](docs/setup-go.md)

#### Rust (RHIVOS Services)

- **Version**: Rust 1.75+ (2021 edition)
- **Location**: `rhivos/`
- **Testing**: `cd rhivos && cargo test --workspace`
- **Linting**: `cd rhivos && cargo clippy --all-targets` (treat warnings as errors in CI)
- **Formatting**: `cd rhivos && cargo fmt` (check with `cargo fmt --check`)
- **Setup guide**: [docs/setup-rust.md](docs/setup-rust.md)

#### Kotlin (Android Automotive App)

- **Version**: Kotlin 2.2+, JDK 17+, Android SDK 34
- **Location**: `android/parking-app/`
- **Testing**: `cd android/parking-app && ./gradlew test`
- **Linting**: `cd android/parking-app && ./gradlew lint`
- **Note**: Requires `ANDROID_HOME` environment variable or `local.properties` file
- **Setup guide**: [docs/setup-android.md](docs/setup-android.md)

#### Flutter/Dart (Companion App)

- **Version**: Flutter 3.16+, Dart 3.2+
- **Location**: `android/companion-app/`
- **Testing**: `cd android/companion-app && flutter test`
- **Linting**: `cd android/companion-app && flutter analyze`
- **Formatting**: `cd android/companion-app && dart format .`
- **Lint rules**: Configured in `analysis_options.yaml`
- **Setup guide**: [docs/setup-flutter.md](docs/setup-flutter.md)
- **MCP Tools**: When working with Flutter/Dart, prefer using the Dart MCP tools (e.g., `run_tests`, `analyze_files`, `dart_format`) over shell commands for better integration.

### Protocol Buffers

All services share protobuf definitions in `proto/`. Generate bindings with:

```bash
make proto          # All languages
make proto-go       # Go only
make proto-rust     # Rust only
make proto-kotlin   # Kotlin only
make proto-dart     # Dart only
```

## Coding Workflow

When instructed to implement a feature, read and understand the guiding documents in `.kiro/specs/{feature-name}/` with

- `requirements.md` - User stories and acceptance criteria
- `design.md` - Architecture and interface definitions
- `tasks.md` - Implementation task list

first, before you start to make any changes.

### Workflow Per Task

When implementing a task, always update the `.kiro/specs/{feature-name}/task.md` document, according to the conventions defined in [steering/requirements-engineering.md](.kiro/steering/requirements-engineering.md).
For Claude: @.kiro/steering/requirements-engineering.md

1. Start from a clean `develop` branch
2. Create feature branch from `develop`: `git checkout -b feature/<task-name> develop`
3. Implement changes

### Before Committing

Run quality checks for the language(s) you modified:

**Go changes:**
```bash
cd backend && go test -short ./... && golangci-lint run ./...
```

**Rust changes:**
```bash
cd rhivos && cargo test --workspace && cargo clippy --all-targets && cargo fmt --check
```

**Kotlin changes:**
```bash
cd android/parking-app && ./gradlew test lint
```

**Flutter/Dart changes:**
```bash
cd android/companion-app && flutter test && flutter analyze && dart format --set-exit-if-changed .
```

**Update docs**: If you changed behavior, update README.md or other docs.

### Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **Run quality gates** (if code changed) - Tests, linters, builds for affected languages
2. **Stage and commit** with descriptive message: `git add . && git commit -m "<type>: <description>"`
3. **Always merge** the changes back to `develop`
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**

- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
- The user is managing multiple agents - unpushed work breaks their coordination workflow

## Makefile Targets

The root `Makefile` provides orchestration across all stacks:

| Target | Description |
|--------|-------------|
| `make all` | Generate protos and build all components |
| `make build` | Build all components |
| `make test` | Run all tests |
| `make clean` | Clean all build artifacts |
| `make proto` | Generate all Protocol Buffer bindings |
| `make infra-up` | Start local development infrastructure |
| `make infra-down` | Stop local development infrastructure |
| `make certs` | Generate development TLS certificates |

