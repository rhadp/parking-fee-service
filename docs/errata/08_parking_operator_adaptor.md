# Errata for Spec 08: PARKING_OPERATOR_ADAPTOR

## 08-ERRATA-1: TS-08-SMOKE-3 Sequence Requires Interleaved Unlock

**Finding:** The test spec TS-08-SMOKE-3 described a sequence where step 3
sends `IsLocked=true` when the signal was already `true` from step 1 (the
manual stop did not change the lock state). In a real Kuksa Databroker
deployment, subscriptions only deliver notifications on value *changes* — a
re-set of the same value produces no notification, so the autonomous session
start in step 3 would never occur.

**Resolution:** The integration test (`TestOverrideThenAutonomousResume`) was
implemented with a corrected sequence:
1. Lock event → autonomous session start
2. Manual StopSession (override)
3. **Unlock event** → no-op (no session active, but resets the lock state)
4. Lock event → autonomous session start resumes
5. Unlock event → autonomous session stop

This matches the intent of 08-REQ-5.E1 and 08-REQ-5.3 (override resumes
autonomous behavior on the next lock/unlock cycle). The mock DATA_BROKER used
in tests delivers all signals unconditionally, so both the corrected sequence
and the original would pass in tests — but the corrected sequence correctly
models real DATA_BROKER change-notification semantics.

**Affected Requirements:** 08-REQ-5.3, 08-REQ-5.E1

**Affected Test:** TS-08-SMOKE-3

---

## 08-ERRATA-2: Rate JSON Key Mismatch (`type` vs `rate_type`)

**Finding:** The PARKING_OPERATOR REST API returns the rate type with the JSON
key `"type"` (e.g. `{"type": "per_hour", "amount": 2.50, "currency": "EUR"}`),
but the Rust `Rate` struct field is named `rate_type` for clarity. Without an
explicit serde rename, JSON deserialization would silently produce an empty
`rate_type` string.

**Resolution:** The `Rate` struct in `session.rs` uses `#[serde(rename = "type")]`
on the `rate_type` field:
```rust
#[serde(rename = "type")]
pub rate_type: String,
```

This ensures correct deserialization from the operator's JSON response while
maintaining a clear Rust field name.

**Affected Requirements:** 08-REQ-2.3

---

## 08-ERRATA-3: DATA_BROKER Retry Count Ambiguity

**Finding:** 08-REQ-3.E3 specifies "retry connection with exponential backoff
(1s, 2s, 4s) up to 5 attempts", but the backoff series (1s, 2s, 4s) contains
only 3 delays, supporting at most 4 total attempts (1 initial + 3 retries).
The requirement is internally self-contradictory.

**Resolution:** The implementation performs up to 5 total attempts using a
5-element delay series (1s, 2s, 4s, 4s, 4s) for the DATA_BROKER connection
retry, satisfying the "up to 5 attempts" clause. For the PARKING_OPERATOR REST
retry, the implementation uses the standard 3-delay backoff (1s, 2s, 4s) = 4
total attempts as specified in the glossary and 08-REQ-2.E1.

**Affected Requirements:** 08-REQ-3.E3

---

## 08-ERRATA-4: `Vehicle.Parking.SessionActive` Requires VSS Overlay

**Finding:** `Vehicle.Parking.SessionActive` is not part of the standard
COVESA VSS signal tree. Kuksa Databroker rejects Set/Subscribe calls for
unregistered signals. The requirements do not explicitly document the need for
a VSS overlay.

**Resolution:** The VSS overlay file at `deployments/vss-overlay.json` already
defines `Vehicle.Parking.SessionActive` as a boolean sensor, along with the
required parent branch node `Vehicle.Parking`. The Kuksa Databroker container
in `deployments/compose.yml` loads this overlay via the `--vss` flag.

**Affected Requirements:** 08-REQ-4.1, 08-REQ-4.2, 08-REQ-4.3
