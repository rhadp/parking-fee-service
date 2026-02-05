# Project Foundation Review (2026-02-05)

**Scope:** `.kiro/specs/project-foundation/{requirements,design,tasks}.md` vs implementation

## Summary

The foundation is largely implemented and aligns with the specification for a proof-of-concept. The monorepo structure, proto definitions, build tooling, infra scaffolding, and container standards are present. Property tests for the stated correctness properties exist and pass locally. I found several spec-to-implementation mismatches and a few doc/command drift issues that will trip up developers unless corrected.

## Tests Run

- `cd tests/property && go test ./...`

Result: pass

## Compliance Highlights

- Requirement 1 (Monorepo structure): present across `rhivos/`, `android/`, `backend/`, `proto/`, `containers/`, `infra/`, `docs/`, `scripts/`.
- Requirement 2 (Proto interfaces + VSS types): all required services and messages exist under `proto/`.
- Requirement 4 (Build system): `Makefile` targets exist for `proto`, `build`, `test`, `infra-up/down`, and container builds. Proto generation script and Buf configs are present.
- Requirement 5 (Communication configuration): endpoint/tls/ports documented in `infra/config/endpoints.yaml` and `infra/config/development.yaml`.
- Requirement 8 (UBI10 base images): all Containerfiles use UBI10 variants with documented rationale and `ghcr.io/rhadp/builder` for Rust/Go build stages.

## Findings

### 1) Kuksa image does not match the required CentOS Automotive container image

- Requirement 3.3 calls for the Eclipse Kuksa Databroker container from the official CentOS Automotive container images.
- `infra/compose/podman-compose.yml` and `docs/local-infrastructure.md` use `ghcr.io/eclipse-kuksa/kuksa-databroker:0.4.4` instead.

Impact: spec mismatch; likely acceptable for PoC but out-of-spec.

References:
- `infra/compose/podman-compose.yml`
- `docs/local-infrastructure.md`

### 2) Container manifest generation is not wired into the build system

- Requirement 4.7 says the build system should generate container manifests during the build process.
- `scripts/generate-manifest.sh` exists, but `make build-containers` does not call it.

Impact: spec mismatch; manual step required.

References:
- `Makefile`
- `scripts/generate-manifest.sh`

### 3) Documentation references Makefile targets that do not exist

- `README.md` and `docs/demo-scenarios.md` reference `make infra-status`, `make logs`, and `make install-demo-adapter`, none of which exist in `Makefile`.

Impact: onboarding friction; demo steps fail.

References:
- `README.md`
- `docs/demo-scenarios.md`
- `Makefile`

### 4) Mock parking operator API paths in docs do not match the implementation

- `containers/mock/parking-operator/app.py` exposes:
  - `POST /sessions/start`, `POST /sessions/stop`, `GET /sessions/{id}`.
- `docs/local-infrastructure.md` uses `/api/v1/sessions` for start/stop.

Impact: demo instructions are incorrect; will cause confusion.

References:
- `containers/mock/parking-operator/app.py`
- `docs/local-infrastructure.md`

### 5) TLS environment variable names drift in docs

- `infra/config/development.yaml` uses `SDV_TLS_SKIP_VERIFY` and `SDV_TLS_DISABLED`.
- `docs/local-infrastructure.md` shows `TLS_SKIP_VERIFY` and `tls.skip_verify` in YAML, which are not in the repo.

Impact: developers will set incorrect env vars.

References:
- `infra/config/development.yaml`
- `docs/local-infrastructure.md`

### 6) Kuksa healthcheck is “NONE” (documented but not actually validating health)

- Requirement 3.6 expects health checks with a test command.
- Compose sets `test: ["NONE"]` for Kuksa and relies on external checks.

Impact: acceptable for PoC, but worth flagging as a limitation.

References:
- `infra/compose/podman-compose.yml`

### 7) Evidence of prior property-test failures is checked in

- `tests/property/testdata/rapid/*.fail` files exist from previous runs.

Impact: not a functional issue, but misleading if tests are expected to be clean.

References:
- `tests/property/testdata/rapid/...`

## Recommendations

### Immediate

1. Align Kuksa image with CentOS Automotive container image or update the requirement to match `ghcr.io/eclipse-kuksa`.
2. Add `make infra-status`, `make logs`, and `make install-demo-adapter` targets or update the docs to match actual commands.
3. Either update `docs/local-infrastructure.md` to use `/sessions/start` and `/sessions/stop`, or change the mock service routes to `/api/v1/sessions` to match the docs.
4. Fix TLS env var names in docs to `SDV_TLS_*` and remove the invalid `tls.skip_verify` snippet.
5. Decide whether to delete or ignore `tests/property/testdata/rapid/*.fail` in git.

### Nice-to-have (PoC-safe)

1. Wire `scripts/generate-manifest.sh` into `make build-containers` so manifests are generated automatically.
2. Replace Kuksa healthcheck with a small sidecar or custom image that can run a gRPC health probe.

## Notes on Spec Interpretation for PoC

- The stub backend services in `backend/` are consistent with “foundation only” scope.
- The `testdata/rapid/*.fail` artifacts do not affect current test runs; `go test ./...` passes.

