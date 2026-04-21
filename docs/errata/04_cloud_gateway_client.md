# Errata: 04_cloud_gateway_client

Spec divergences and ambiguities discovered during implementation.

---

## E1 — NATS Reconnection Backoff: 3-delay spec vs 4-delay test

**Source:** Skeptic major finding `53d379af`  
**Affects:** [04-REQ-2.2], TS-04-15

**Problem:**  
REQ-2.2 states "retry with exponential backoff (1s, 2s, 4s) for up to 5 attempts", which names
only three delay values. Five attempts require four inter-attempt delays. TS-04-15 (integration
test) implies delays of 1s, 2s, 4s, 8s — a four-delay sequence totalling 15 s before giving up.

**Resolution:**  
Implement four delays (1s, 2s, 4s, 8s) as described by TS-04-15. This is consistent with a
standard doubling backoff and matches the integration test expectations. The "1s, 2s, 4s" list
in REQ-2.2 is treated as the first three intervals; the fourth (8s) is implied by the doubling
pattern.

---

## E2 — Self-Registration Trigger: NATS-connected vs post-DATA_BROKER

**Source:** Skeptic major findings `26686601`, `bd3f2a7c`  
**Affects:** [04-REQ-4.1], [04-REQ-9.1], [04-REQ-9.2]

**Problem:**  
REQ-4.1 states self-registration is published "WHEN the service has connected to NATS" (step 2
of startup). REQ-9.1 places registration at step 4, after DATA_BROKER connection (step 3).
These are contradictory: if DATA_BROKER fails (step 3), under REQ-4.1 the registration was
already published, but under REQ-9.1 + REQ-9.2 the service exits without publishing it.

Additionally, REQ-9.1 step 5 ("begin processing commands and telemetry") requires NATS and
DATA_BROKER subscriptions that could fail. Publishing registration at step 4 before
subscriptions are established means a subscription failure leaves the cloud with a false
"online" signal — the service exits but the CLOUD_GATEWAY believes the vehicle is available.

**Resolution:**  
The implementation defers self-registration until after all subscriptions are established:
config → NATS connect → DATA_BROKER connect → subscribe all channels → publish registration →
begin processing. This ensures the CLOUD_GATEWAY only sees a "vehicle online" message when
the service is fully operational (all connections established, all subscriptions active).

This reinterprets REQ-9.1 step 5 as including subscriptions, and moves registration to
after subscriptions succeed. The five spec steps map to six implementation steps:
(1) config, (2) NATS connect, (3) DATA_BROKER connect, (4) subscribe channels,
(5) publish registration, (6) begin processing loop.

---

## E3 — REQ-7.2 Validation Gap: Missing required response fields

**Source:** Skeptic major finding `2c5597a5`  
**Affects:** [04-REQ-7.1], [04-REQ-7.2], [04-REQ-7.E1]

**Problem:**  
REQ-7.2 requires the relayed payload to contain `command_id`, `status`, and `timestamp`, but
the only validation required is REQ-7.E1 (valid JSON check). If DATA_BROKER emits valid JSON
lacking these fields, the payload passes REQ-7.E1 but violates REQ-7.2. No spec edge-case
addresses this path.

**Resolution:**  
The implementation relays DATA_BROKER response JSON verbatim after passing the valid-JSON check
(REQ-7.E1). Field-presence validation is NOT added — REQ-7.2 is treated as a specification of
what LOCKING_SERVICE MUST produce (not as a filter enforced by CLOUD_GATEWAY_CLIENT). This
interpretation is consistent with the "relay verbatim" language in REQ-7.1 and the absence of
any specified action for missing fields in REQ-7.2.

---

## E4 — REQ-6.4 vs Vec<String> for doors array element types

**Source:** Skeptic minor finding `cc839770`  
**Affects:** [04-REQ-6.4]

**Problem:**  
REQ-6.4 prohibits validation of individual door values. Using `Vec<String>` in `CommandPayload`
implicitly rejects non-string door elements (e.g., integers) via serde deserialization, which
constitutes element-type validation.

**Resolution:**  
`CommandPayload.doors` uses `Vec<serde_json::Value>` rather than `Vec<String>`. This accepts
any JSON value type in the doors array, fully delegating all door-value semantics to
LOCKING_SERVICE as intended by REQ-6.4.
