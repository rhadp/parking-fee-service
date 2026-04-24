# Errata: 02_data_broker (auto-generated)

**Spec:** 02_data_broker
**Date:** 2026-04-23
**Status:** Active
**Source:** Auto-generated from reviewer blocking findings

## Findings

### Finding 1

**Summary:** [critical] Image version is contradictory across all spec artifacts. 02-REQ-1.1 mandates image tag `0.6.1`; 02-REQ-1.2 Acceptance Criterion 2 states the container SHALL report "version 0.5.1 in its startup logs" — a 0.6.1 image cannot report 0.5.1. The design Technology Stack table says version `0.5.1`, the Design Definition of Done says "pinned image (`0.5.1`)", and the actual compose.yml uses `0.5.0`. Three different version numbers appear across requirements, design, and implementation. Implementers cannot satisfy all acceptance criteria simultaneously.
**Requirement:** 02-REQ-1.1
**Task Group:** 1

### Finding 2

**Summary:** [critical] The design specifies `kuksa.val.v2` gRPC API with methods `Get`, `Set`, `GetMetadata`, and `Subscribe`. However, the Kuksa Databroker container (0.5.x / 0.6.x) exposes `kuksa.val.v2` with a completely different method set: `GetValue`, `GetValues`, `PublishValue`, `ListMetadata`, `Actuate`, and `Subscribe`. Calling the methods named in the design and test_spec against the running container returns empty responses with no error instead of a gRPC error — meaning all live integration tests (TS-02-4 through TS-02-11, all property tests, all smoke tests) will silently return incorrect results rather than failing cleanly. This makes every signal set/get/metadata/subscribe test non-functional when the real container is running.
**Requirement:** 02-REQ-5.2
**Task Group:** 1

### Finding 3

**Summary:** [major] 02-REQ-10.E1 requires that after a subscriber disconnects and reconnects, it can re-subscribe and receive subsequent updates without missing the current value. The coverage matrix maps this requirement to TS-02-10, but TS-02-10 only tests initial subscription delivery — it subscribes once, receives one update, and asserts the value. No test in the test_spec exercises disconnect, reconnect, or the "without missing the current value" guarantee. This acceptance criterion has no test coverage.
**Requirement:** 02-REQ-10.E1
**Task Group:** 1

### Finding 4

**Summary:** [major] 02-REQ-2.1 Acceptance Criterion 3 requires command args to include `--address 0.0.0.0:55555` (combined host:port form), and 02-REQ-3.1 Acceptance Criterion 1 requires `--uds-path /tmp/kuksa-databroker.sock`. The actual Kuksa Databroker binary does not support these flag forms; it uses `--address 0.0.0.0 --port 55555` (separate flags) and `--unix-socket` (not `--uds-path`). Tests asserting the exact strings from the requirements will fail against the real compose.yml configuration, or the compose.yml cannot be written to satisfy both the binary's actual CLI and the spec's acceptance criteria simultaneously.
**Requirement:** 02-REQ-2.1
**Task Group:** 1

### Finding 5

**Summary:** [major] All UDS integration tests (TS-02-2, TS-02-7, TS-02-8, TS-02-9, TS-02-11, TS-02-SMOKE-3, TS-02-SMOKE-4) specify the path `unix:///tmp/kuksa-databroker.sock`, which is the container-internal path. The compose.yml bind-mounts `/tmp/kuksa` (host) to `/tmp` (container), making the socket accessible on the host at `/tmp/kuksa/kuksa-databroker.sock` — not at `/tmp/kuksa-databroker.sock`. Tests executed from the host (as specified in the design: "Tests are implemented in Go under tests/databroker/") will fail to connect via UDS because the socket path does not exist at the expected host location.
**Requirement:** 02-REQ-3.2
**Task Group:** 1

### Finding 6

**Summary:** [major] The design Compose service interface specifies service name `databroker`, but the actual compose.yml uses `kuksa-databroker`. The edge case tests TS-02-E2 and TS-02-E3 reference `podman_compose_up("databroker")` and `assert_container_not_running("databroker")`, and TS-02-SMOKE-1 references `podman_compose_up("databroker", ...)`. All container lifecycle operations in the test spec will target the wrong service name and will not control or observe the actual databroker container.
**Requirement:** 02-REQ-6.E1
**Task Group:** 1

### Finding 7

**Summary:** [major] The design Module Responsibilities table lists the overlay file as `deployments/vss/overlay.vspec` (VSPEC YAML format), and the design architecture diagram annotates the overlay mount with "--vss flag". The actual file is `deployments/vss-overlay.json` (JSON format) at a different path, and the compose.yml uses `--metadata` (not `--vss`). The Glossary in requirements.md also calls it "A YAML file". This three-way inconsistency (path, format, CLI flag) between the design, requirements glossary, and implementation means implementers following the design will produce a non-functional configuration.
**Requirement:** 02-REQ-6.4
**Task Group:** 1
