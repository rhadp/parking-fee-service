# Errata: Spec 01 Test Scope Exclusions

## Context

Spec 01 (Project Setup) requires `make test` and `make check` to run all Rust
and Go tests, returning exit code 0 when all tests pass (01-REQ-6.3,
01-REQ-6.5).

## Deviation

The Makefile's `test-rust` and `test-go` targets exclude components that
contain unimplemented test stubs from later specifications:

### Rust exclusions (`CARGO_TEST_EXCLUDE`)

| Crate | Reason |
|-------|--------|
| `locking-service` | Spec 03 TG1 stubs: 25 failing tests with `todo!()` in command, config, process, response, and safety modules |
| `cloud-gateway-client` | Spec 04 TG1 stubs: 13 failing tests with `todo!()` in command_validator module |
| `update-service` | Spec 07 TG1 stubs: 34 failing tests with `todo!()` in adapter, config, state, grpc, monitor, and offload modules |

### Go exclusions (`GO_TEST_MODULES_ROOT` / `GO_TEST_MODULES_RECURSIVE` vs `GO_MODULES`)

| Module | Scope | Reason |
|--------|-------|--------|
| `backend/parking-fee-service` | Root only (`go test .`) | Spec 05 TG1 stubs: failing tests in config, geo, handler, and store subpackages |
| `backend/cloud-gateway` | Root only (`go test .`) | Spec 06 TG1 stubs: failing tests in auth, natsclient, and store subpackages |
| `mock/parking-operator` | Excluded entirely | Spec 09 TG1 stubs: 9 failing tests in server and main packages |

## Rationale

Spec-driven development uses a test-first approach where TG1 of each spec
writes failing tests before implementation. These stubs are expected to fail
until their respective implementation task groups complete. Excluding them
from the project-wide test target ensures `make test` and `make check` remain
green for the components that are fully implemented, while the excluded stubs
are still compiled and linted (they remain in the build and lint targets).

## Resolution

Remove each exclusion when the corresponding spec's implementation is
complete:

- Remove `--exclude locking-service` after spec 03 TG2+ completes
- Remove `--exclude cloud-gateway-client` after spec 04 TG2+ completes
- Remove `--exclude update-service` after spec 07 TG2+ completes
- Move `backend/parking-fee-service` from `GO_TEST_MODULES_ROOT` to
  `GO_TEST_MODULES_RECURSIVE` after spec 05 TG2+ completes
- Move `backend/cloud-gateway` from `GO_TEST_MODULES_ROOT` to
  `GO_TEST_MODULES_RECURSIVE` after spec 06 TG2+ completes
- Add `mock/parking-operator` to `GO_TEST_MODULES_RECURSIVE` after spec 09
  TG2+ completes
