# Errata: 01_makefile_test_scope

Divergence from spec 01 requirements for `make test` scope.

---

## E1 — test-rust excludes task-group-1 stub crates

**Affects:** [01-REQ-6.3], TS-01-20

**Problem:**  
REQ-6.3 mandates `make test` runs `cargo test --workspace` for all Rust crates.
However, the spec-driven TDD workflow requires task group 1 agents to write
*intentionally failing* tests before implementations exist. Crates in this state
have `todo!()` stubs that panic at runtime, making `make test` fail before their
implementation task groups run.

**Affected crates (currently excluded from test-rust):**

| Crate | Reason |
|-------|--------|
| `mock-sensors` | Spec 09 task group 1 sensor arg tests fail (pre-implementation) |
| `cloud-gateway-client` | Spec 04 task group 1 stubs (validators, telemetry) fail |

**Resolution:**  
`make test` (`test-rust`) excludes crates whose task group 1 tests are written but
not yet implemented. Each crate is re-included once its implementation task group
completes and all its tests pass. This is the expected incremental TDD workflow;
the full `make check` (lint + compile-only) still covers these crates.

**Re-include when:**  
- `mock-sensors`: when spec 09 task group 2+ completes sensor implementations
- `cloud-gateway-client`: when spec 04 task group 2+ completes command validator
  and telemetry implementations
