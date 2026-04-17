# Errata: Makefile test-rust and test-go Scope

**Related Spec:** 01_project_setup (task group 5)
**Date:** 2026-04-17

## Situation

The root `Makefile` created in spec 01 task group 5 scopes the `test-rust` and
`test-go` targets to only the workspace members/modules whose tests currently
pass. Several workspace members have pre-existing failing tests from earlier spec
group 1 "RED phase" (failing tests written before implementation exists).

## Divergence

**01-REQ-6.3** states:
> WHEN `make test` is invoked, THEN it SHALL run `cargo test` in `rhivos/` and
> `go test ./...` for all Go modules, returning exit code 0 when all tests pass.

The Makefile cannot unconditionally run `cargo test --workspace` or
`go test parking-fee-service/...` because the following workspace members have
unimplemented tests that panic with `todo!()`:

### Rust workspace members excluded from `test-rust`

| Crate | Reason excluded |
|-------|----------------|
| `locking-service` | ~~41 spec-03 tests failing (todo! stubs)~~ **Implemented in spec 03 task group 2-3; now included in test-rust** |
| `update-service` | 35 spec-07 tests failing (todo! stubs); was incorrectly included before spec 03 TG5 |
| `parking-operator-adaptor` | 26 spec-08 tests failing (todo! stubs); was incorrectly included before spec 03 TG5 |
| `cloud-gateway-client` | 24 spec-04 tests failing (todo! stubs) |
| `mock-sensors` (integration tests) | 3 spec-09 sensor tests failing; only `--lib` is run |

### Go modules excluded from `test-go`

| Module | Reason excluded |
|--------|----------------|
| `mock/parking-operator` | 6 spec-09 tests failing (stub HTTP server) |
| `backend/parking-fee-service/config` | spec-05 task group 1 stub tests |
| `backend/parking-fee-service/geo` | spec-05 task group 1 stub tests |
| `backend/parking-fee-service/handler` | spec-05 task group 1 stub tests |
| `backend/parking-fee-service/model` | spec-05 task group 1 stub tests |
| `backend/parking-fee-service/store` | spec-05 task group 1 stub tests |
| `backend/cloud-gateway/auth` | spec-06 task group 1 stub tests |
| `backend/cloud-gateway/config` | spec-06 task group 1 stub tests |
| `backend/cloud-gateway/handler` | spec-06 task group 1 stub tests |
| `backend/cloud-gateway/model` | spec-06 task group 1 stub tests |
| `backend/cloud-gateway/natsclient` | spec-06 task group 1 stub tests |
| `backend/cloud-gateway/store` | spec-06 task group 1 stub tests |

## Resolution (Updated in spec 03 task group 5)

`test-rust` runs:
- `cargo test -p locking-service` (fully implemented in spec 03 task groups 2-3; all 39 tests pass)
- `cargo test -p mock-sensors --lib` (only spec-01 lib placeholder test, pass)

`update-service` and `parking-operator-adaptor` were removed from `test-rust` because they
still have `todo!()` stub implementations. The original Makefile incorrectly included them.

`test-go` runs modules explicitly (root packages only, without `...` recursion
for the two backend modules that now have failing sub-packages from spec 05 and
spec 06 task group 1 stubs):
- `backend/parking-fee-service` (root only — spec-05 sub-packages have stub tests)
- `backend/cloud-gateway` (root only — spec-06 sub-packages have stub tests)
- `mock/parking-app-cli/...`, `mock/companion-app-cli/...`
- `tests/setup/...`

Additionally, `update-service/src/main.rs` was updated in spec 03 task group 5 to include
the skeleton flag-handling code (`starts_with('-')`, `eprintln!`, `process::exit(1)`)
required by `TestRustSkeletonsHandleUnknownFlags` (TS-01-E4 / 01-REQ-4.E1).

## Impact

- `make test` and `make check` pass for all currently-implemented specs
- Full workspace test (`cargo test --workspace`) still fails until specs 04, 07, 08, 09 implement their
  stub functions
- Pre-existing failures are tracked in:
  - `docs/errata/01_skeleton_vs_spec09_sensors.md` (sensor integration tests)
  - This file (Makefile scope)
