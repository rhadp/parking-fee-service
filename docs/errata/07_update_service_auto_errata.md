# Errata: 07_update_service (auto-generated)

**Spec:** 07_update_service
**Date:** 2026-04-23
**Status:** Active
**Source:** Auto-generated from reviewer blocking findings

## Findings

### Finding 1

**Summary:** [critical] No coordination between the container monitor task and RemoveAdapter (or the single-adapter stop) is specified, producing a race condition. When RemoveAdapter calls `podman stop` on a RUNNING container (design.md Path 2, step 3), the background `podman wait` task (Path 4) simultaneously detects the container exit and attempts a RUNNING→STOPPED or RUNNING→ERROR transition. If the monitor fires first and transitions the adapter to STOPPED, the adapter becomes eligible for the offload timer even though RemoveAdapter intends to delete it immediately. Conversely, if RemoveAdapter removes the adapter from state first, the monitor's transition call hits a missing entry—acceptable but unspecified. Neither the requirements nor design specifies any cancellation, sentinel flag, or lock that prevents these two concurrent writers from racing over the same adapter.
**Requirement:** 07-REQ-9.2
**Task Group:** 1

### Finding 2

**Summary:** [critical] The install flow's single-adapter check is not atomic with the subsequent state creation, violating Property 2 under concurrent load. Design.md install path step 4 reads 'Check if another adapter is RUNNING; if so, stop it first,' then step 5 creates the new adapter entry. If two InstallAdapter RPCs arrive simultaneously, both pass the check (no running adapter found), both proceed to create entries, and both reach RUNNING, violating the at-most-one-RUNNING invariant. The design proposes `DashMap` or `Arc<Mutex<HashMap>>`, neither of which makes the check-stop-create sequence atomic. No requirement or design note mandates that this entire sequence be executed under a single lock hold.
**Requirement:** 07-REQ-2.1
**Task Group:** 1

### Finding 3

**Summary:** [major] Property 3 in design.md (and the corresponding TS-07-P3 valid-transitions set) includes `(Stopped, Running)` as a valid state transition, but the ASCII state machine diagram in design.md shows no STOPPED→RUNNING edge and no requirement, RPC, or execution path ever causes this transition. 07-REQ-8.1 lists the exhaustive set of emitted transitions and does not include STOPPED→RUNNING. This inconsistency within the design document will cause property tests implementing TS-07-P3 to accept a transition that is unreachable in any correct execution, weakening the invariant being tested.
**Requirement:** 07-REQ-8.1
**Task Group:** 1

### Finding 4

**Summary:** [major] The Remove Adapter execution path (design.md Path 2) stops the container with `podman stop` if running (step 3), then immediately executes `podman rm` and removes the adapter from state (steps 4–5), without specifying any RUNNING→STOPPED state transition or event emission. 07-REQ-8.1 requires an event for every RUNNING→STOPPED transition. An explicit removal that skips this event means subscribers observing a WatchAdapterStates stream never see the adapter leave RUNNING state—it simply disappears. This gap is not acknowledged as intentional anywhere in the spec.
**Requirement:** 07-REQ-8.1
**Task Group:** 1

### Finding 5

**Summary:** [major] The `registry_url` and `container_storage_path` fields mandated by 07-REQ-7.2 have no documented purpose in any requirement, execution path, podman command, or module interface across the entire spec. No requirement reads or uses either field after loading. Implementing them as inert config fields provides no testable behavior, and no test verifies that they influence any outcome. The spec should either document what these fields control or remove them.
**Requirement:** 07-REQ-7.2
**Task Group:** 1

### Finding 6

**Summary:** [major] `tokio::sync::broadcast` drops messages to receivers that fall behind the internal ring buffer (they receive `RecvError::Lagged`). 07-REQ-3.3 and 07-REQ-8.3 guarantee all active subscribers receive every event, but this guarantee is violated whenever a subscriber is slow and the buffer fills. The spec specifies neither the channel buffer size nor any required handling of the lagged condition (e.g., closing the lagged stream with an error, or using an unbounded channel). Property test TS-07-P4 ('all N subscribers receive identical event sequences') is therefore untestable under load without this specification.
**Requirement:** 07-REQ-3.3
**Task Group:** 1
