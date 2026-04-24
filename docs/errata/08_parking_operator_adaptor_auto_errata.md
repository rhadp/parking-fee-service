# Errata: 08_parking_operator_adaptor (auto-generated)

**Spec:** 08_parking_operator_adaptor
**Date:** 2026-04-23
**Status:** Active
**Source:** Auto-generated from reviewer blocking findings

## Findings

### Finding 1

**Summary:** [critical] The `SessionStatus` proto message returned by the `GetStatus` RPC contains only {session_id, active, start_time, zone_id} and has no `rate` field. Requirement 08-REQ-1.4 mandates that GetStatus return rate information alongside those fields. Test TS-08-4 asserts `resp.rate.rate_type == "per_hour"` and `resp.rate.amount`, which cannot be satisfied by the current proto contract. The proto definition and the requirement are irreconcilable without a schema change.
**Requirement:** 08-REQ-1.4
**Task Group:** 1

### Finding 2

**Summary:** [major] No test case covers the scenario where `stop_session` REST call exhausts all retries. Requirement 08-REQ-2.E1 states retry logic applies to any PARKING_OPERATOR REST call failure, but TS-08-E3 and TS-08-E4 only exercise `start_session` failure paths. The behavior when `stop_session` fails all retries — session remains active in memory, UNAVAILABLE returned to caller — is specified but not tested, leaving a systematic gap in the stop execution path.
**Requirement:** 08-REQ-2.E1
**Task Group:** 1

### Finding 3

**Summary:** [major] TS-08-P6 (Sequential Event Processing) specifies comparing `result_sequential == result_concurrent` after delivering N events concurrently. This comparison is not a valid correctness invariant: the outcome of concurrent event delivery is inherently non-deterministic (scheduler-dependent), so no fixed "concurrent result" exists to compare against. The property cannot be implemented as a deterministic, repeatable test and provides no actual safety guarantee for 08-REQ-9.1.
**Requirement:** 08-REQ-9.1
**Task Group:** 1

### Finding 4

**Summary:** [major] The design architecture flowchart shows a `GET /parking/status/{id}` REST call to the PARKING_OPERATOR, and the `OperatorClient` interfaces section defines a corresponding `async fn get_status(&self, session_id: &str) -> Result<StatusResponse, OperatorError>` method. Neither appears in any requirement, acceptance criterion, execution path, or test case. This creates an orphaned design element with no requirement backing — the method either becomes dead code or an unanticipated, untested code path.
**Task Group:** 1

### Finding 5

**Summary:** [major] `StartSessionRequest` in the proto definition includes a `vehicle_id` field alongside `zone_id`. Requirement 08-REQ-1.2 describes `StartSession(zone_id)` as taking only zone_id. Requirement 08-REQ-3.3 specifies that autonomous starts use vehicle_id from the `VEHICLE_ID` environment variable. For manual gRPC `StartSession` calls, the spec does not specify whether the adaptor should use the vehicle_id from the gRPC request field or from the env var, meaning the vehicle_id sent to the PARKING_OPERATOR for manual sessions is implementation-defined.
**Requirement:** 08-REQ-1.2
**Task Group:** 1

### Finding 6

**Summary:** [major] `StopSessionRequest` in the proto definition includes a `session_id` field (field 1), but 08-REQ-1.3 describes `StopSession()` as taking no input parameters — the adaptor is supposed to stop the active session using internal state. The spec does not specify whether the gRPC-provided session_id should be used, ignored, or validated against internal state, leaving this ambiguity unresolved between the wire contract and the requirement.
**Requirement:** 08-REQ-1.3
**Task Group:** 1
