# Agent-Fox Memory

## Gotchas

- Use regexp.QuoteMeta() when embedding user input into regex patterns to prevent regex injection and ensure literal string matching. _(spec: 01_project_setup, confidence: 0.90)_

## Patterns

- A steering.md file in .specs/ directory can be used to define directives that influence agent behavior across all sessions in a repository. _(spec: 01_project_setup, confidence: 0.60)_
- A `.specs/steering.md` file can be used to document and communicate steering directives that influence agent behavior across all sessions and tasks in a repository. _(spec: 05_parking_fee_service, confidence: 0.60)_
- A steering.md file can be created in .specs/ directory to define directives that influence agent behavior across all sessions in a repository. _(spec: 06_cloud_gateway, confidence: 0.60)_
- The steering.md file uses a placeholder marker comment that the system recognizes to determine if the file contains actual directives or is still empty. _(spec: 04_cloud_gateway_client, confidence: 0.60)_
- Verification tests should be written before implementing the actual project structure, establishing a clear specification through failing tests that guide implementation. _(spec: 01_project_setup, confidence: 0.90)_
- A comprehensive project setup test suite should cover multiple dimensions: directory structure, language-specific workspace configurations (Rust, Go), build automation (Makefile), infrastructure config, and proto file validation. _(spec: 01_project_setup, confidence: 0.60)_
- Use filepath.Walk with a helper function to search for test markers (#[test] in Rust, func Test in Go) across directory trees when validating test presence in multiple modules. _(spec: 01_project_setup, confidence: 0.90)_
- When testing file existence and content validation across a monorepo, use a repoRoot() helper that walks up from cwd to find .git to locate the repository root, enabling tests to run from any working directory. _(spec: 01_project_setup, confidence: 0.90)_
- Regex patterns for validating configuration files should use (?m) flag for multiline mode and ^/$ anchors to match at line boundaries rather than string boundaries. _(spec: 01_project_setup, confidence: 0.90)_
- Validate JSON files by unmarshaling into an empty interface (var parsed any) to detect malformed JSON without requiring a specific struct schema. _(spec: 01_project_setup, confidence: 0.90)_
- Helper functions like pathExists() and readFileContent() reduce boilerplate and improve test readability when repeated across many validation tests. _(spec: 01_project_setup, confidence: 0.90)_
- Check file size > 0 when validating configuration files to ensure they are not empty placeholders. _(spec: 01_project_setup, confidence: 0.60)_
- Go test modules can be organized in a dedicated tests/ directory with their own go.mod file separate from main application modules. _(spec: 08_parking_operator_adaptor, confidence: 0.90)_
- The repoRoot() helper function can reliably locate repository root by walking up the directory tree until finding .git, useful for tests that need to reference absolute paths. _(spec: 08_parking_operator_adaptor, confidence: 0.90)_
- Test verification should check for presence of both test files (_test.go for Go, #[test] for Rust) and actual test functions within those files. _(spec: 08_parking_operator_adaptor, confidence: 0.90)_
- JSON configuration files should be validated for syntactic correctness using unmarshal/parse operations in addition to checking for required content. _(spec: 08_parking_operator_adaptor, confidence: 0.90)_
- Helper functions in Go test packages (like pathExists, readFileContent) reduce duplication and improve test readability when verifying filesystem state. _(spec: 08_parking_operator_adaptor, confidence: 0.90)_
- Use filepath.Walk() to recursively search directories for files matching a pattern (e.g., finding .rs files with #[test] attributes). _(spec: 07_update_service, confidence: 0.90)_
- Go test verification should check for both file existence (_test.go) and function presence (func Test*) rather than relying on file naming alone. _(spec: 07_update_service, confidence: 0.90)_
- Use regexp.MustCompile with (?m) flag for multiline regex matching in file content validation, particularly for target definitions in Makefiles. _(spec: 07_update_service, confidence: 0.90)_
- Repository root detection in tests should walk up the directory tree looking for .git rather than assuming a fixed relative path. _(spec: 07_update_service, confidence: 0.90)_
- JSON validation in Go tests should use json.Unmarshal with an empty interface{} type to verify valid JSON without needing to know the schema. _(spec: 07_update_service, confidence: 0.90)_
- Specification verification tests should be organized in a separate tests/setup/ directory with its own go.mod module for monorepo projects. _(spec: 07_update_service, confidence: 0.60)_
- Use filepath.Walk with a closure that checks for file extensions to recursively search for specific file types across a directory tree. _(spec: 09_mock_apps, confidence: 0.90)_
- Repository root detection via walking up the directory tree until finding .git is a reliable pattern for tests that need absolute paths. _(spec: 09_mock_apps, confidence: 0.90)_
- Regex patterns in Go should use `(?m)^` anchor syntax for multiline matching at the start of lines in file content. _(spec: 09_mock_apps, confidence: 0.90)_
- JSON validation in Go tests can use json.Unmarshal into an empty `any` variable to check if content is valid JSON without parsing specific structure. _(spec: 09_mock_apps, confidence: 0.90)_
- os.ReadDir returns entries that should be checked with entry.Name() suffix matching rather than filepath operations for simple file type detection. _(spec: 09_mock_apps, confidence: 0.60)_
- Rust monorepo workspaces are configured with a root Cargo.toml that declares multiple crates as members, enabling shared dependency management and coordinated builds across the workspace. _(spec: 01_project_setup, confidence: 0.90)_
- Go monorepos use a go.work file at the root to coordinate multiple Go modules, allowing local development across modules without requiring published versions. _(spec: 01_project_setup, confidence: 0.90)_
- When implementing multi-language project skeletons, verify that existing implementations in other languages (e.g., Go) already satisfy requirements before duplicating work across all language variants. _(spec: 01_project_setup, confidence: 0.60)_
- Proto files should be validated using protoc parsing and automated test suites (like TestProtoFilesValid) to ensure correctness before integration. _(spec: 01_project_setup, confidence: 0.90)_
- When implementing mock apps with multiple language integrations (Rust, Go), organize tests into clear categories: unit tests for individual components, integration tests for CLI/server interactions, and error case tests (E-prefixed) for validation and failure scenarios. _(spec: 09_mock_apps, confidence: 0.60)_
- When implementing a feature with TDD, create stub modules with type definitions and todo!() implementations first, then write failing tests against those stubs to drive development. _(spec: 03_locking_service, confidence: 0.90)_
- Using ignored property tests (#[ignore]) alongside regular unit tests allows you to defer property-based testing while focusing on core functionality validation in early development stages. _(spec: 03_locking_service, confidence: 0.60)_
- Creating a MockBrokerClient test helper early enables isolated unit testing of components that depend on external broker integration without implementing the actual broker logic. _(spec: 03_locking_service, confidence: 0.90)_
- Organizing test coverage across multiple concern areas (command parsing, safety constraints, orchestration, formatting, config) in a single task group helps ensure comprehensive specification coverage from the start. _(spec: 03_locking_service, confidence: 0.60)_
- A root Makefile should include standard targets: build, test, clean, proto, infra-up, infra-down, and check for consistent project development workflow. _(spec: 01_project_setup, confidence: 0.90)_
- VSS (Vehicle Signal Specification) configuration can be customized using overlays to define custom signals alongside standard signals. _(spec: 01_project_setup, confidence: 0.60)_
- Infrastructure setup verification should be validated through automated tests (17 setup verification tests in this case) that confirm all services are properly configured. _(spec: 01_project_setup, confidence: 0.90)_
- When iterating over multiple modules in a Makefile loop, use `$(CURDIR)` to return to the root directory after each cd to avoid relative path issues. _(spec: 03_locking_service, confidence: 0.90)_
- Proto code generation should validate that required tools (protoc, protoc-gen-go, protoc-gen-go-grpc) are installed before running compilation. _(spec: 03_locking_service, confidence: 0.90)_
- NATS server configuration should set reasonable limits like max_payload and max_connections to prevent resource exhaustion in development environments. _(spec: 03_locking_service, confidence: 0.60)_
- The `check` target should combine linting (cargo clippy, go vet) with build and test execution as a comprehensive quality gate. _(spec: 03_locking_service, confidence: 0.60)_
- Makefile should handle cargo clippy failures gracefully by falling back to cargo check to prevent CI/CD blockage on lint-only issues. _(spec: 03_locking_service, confidence: 0.60)_
- Task group organization can be used to segment implementation work, with each group containing related modules (e.g., config, command, safety, response in one group; process in another). _(spec: 03_locking_service, confidence: 0.60)_
- Unit tests and property tests can be run separately and may have different pass rates; failures in one module group don't necessarily block other groups from passing their tests. _(spec: 03_locking_service, confidence: 0.60)_

## Decisions

- When testing Go module structure, distinguish between modules that require main.go (executable modules) and those that don't (test/library modules) to avoid false failures. _(spec: 01_project_setup, confidence: 0.90)_

## Conventions

- A steering.md file can be created at the repository root to define directives that influence agent behavior across all sessions and skills working on the codebase. _(spec: 03_locking_service, confidence: 0.60)_
- A steering.md file can be created at .specs/steering.md to define directives that influence agent behavior across all sessions and skills working on the repository. _(spec: 02_data_broker, confidence: 0.60)_
- The steering.md file contains a placeholder marker and comment block that should be removed once actual directives are added, or the system ignores the file if it contains only the placeholder. _(spec: 02_data_broker, confidence: 0.60)_
- The steering.md file uses a placeholder marker comment that should be removed once actual directives are added to the file. _(spec: 01_project_setup, confidence: 0.60)_
- Steering directives in `.specs/steering.md` should be removed or replaced once actual directives are added; the system ignores files containing only placeholder markers and comments. _(spec: 05_parking_fee_service, confidence: 0.60)_
- The steering.md file uses a placeholder marker comment that the system recognizes to distinguish between initialized and uninitialized steering files. _(spec: 06_cloud_gateway, confidence: 0.60)_
- A steering.md file can be created in .specs/ directory to define repository-wide directives that influence agent behavior across all sessions and skills. _(spec: 04_cloud_gateway_client, confidence: 0.60)_
- Store test specification references (like TS-01-1, TS-01-2) and requirement mappings (01-REQ-1.1) as code comments near test functions for traceability and maintainability. _(spec: 01_project_setup, confidence: 0.90)_
- Workspace configuration files (go.work for Go, Cargo.toml for Rust) should explicitly reference all modules/crates to ensure they are part of the workspace. _(spec: 08_parking_operator_adaptor, confidence: 0.90)_
- Proto files should declare syntax = "proto3", include package declarations, and specify go_package options for proper code generation. _(spec: 08_parking_operator_adaptor, confidence: 0.90)_
- Makefile targets should be defined at the start of a line followed by a colon; regex pattern matching can verify their presence reliably. _(spec: 08_parking_operator_adaptor, confidence: 0.90)_
- Helper functions in Go tests should use t.Helper() to exclude them from test call stacks and improve error reporting clarity. _(spec: 07_update_service, confidence: 0.90)_
- Helper functions in Go tests should use t.Helper() to ensure stack traces point to the test code, not the helper. _(spec: 09_mock_apps, confidence: 0.90)_
- Test files should use t.Errorf for multiple validation failures and t.Fatal for critical preconditions to allow comprehensive error reporting. _(spec: 09_mock_apps, confidence: 0.90)_
- Binary targets in Rust crates are defined in the src/bin/ directory alongside the library code, allowing a single crate to produce multiple executable outputs. _(spec: 01_project_setup, confidence: 0.90)_
- Go module initialization requires a main.go file in the root of each module directory, with accompanying _test.go files for unit tests following the naming convention. _(spec: 01_project_setup, confidence: 0.90)_
- Rust skeleton binaries should reject unknown arguments by printing a usage message to stderr and exiting with a non-zero exit code. _(spec: 01_project_setup, confidence: 0.90)_
- Rust crates should include #[test] it_compiles placeholder tests as part of skeleton setup to verify basic compilation. _(spec: 01_project_setup, confidence: 0.90)_
- Proto files require proto3 syntax declaration, package declarations, go_package options, message type definitions, and RPC service definitions to be valid and parseable by protoc. _(spec: 01_project_setup, confidence: 0.90)_
- In TDD workflows, the red phase should include failing tests across all six mock tools before proceeding to implementation, ensuring comprehensive test coverage from the start. _(spec: 09_mock_apps, confidence: 0.60)_
- NATS server runs on port 4222 and Kuksa Databroker runs on port 55556 in the Docker Compose infrastructure setup. _(spec: 01_project_setup, confidence: 0.90)_
- Makefiles should use `.PHONY` declarations for all non-file targets to prevent conflicts with similarly-named files in the directory. _(spec: 03_locking_service, confidence: 0.90)_
- Compose files should use volume mounts with relative paths (./file) and container-internal paths to allow flexibility in deployment location. _(spec: 03_locking_service, confidence: 0.60)_
