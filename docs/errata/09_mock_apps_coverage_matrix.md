# Errata: Spec 09 — Coverage Matrix Misalignments

**Spec:** 09_mock_apps
**Status:** Accepted — tests exist, matrix references are incorrect

## Summary

The coverage matrix in `test_spec.md` contains two misaligned mappings where
a requirement is mapped to a test spec entry that tests a different tool.
The actual test coverage exists but is not anchored to the referenced test
spec ID.

## Misalignment 1: 09-REQ-4.E1

| Field       | Matrix says          | Actual                           |
|-------------|----------------------|----------------------------------|
| Requirement | 09-REQ-4.E1          | parking-app-cli missing flags    |
| Mapped to   | TS-09-E1             | Location Sensor Missing Args     |
| Real test   | —                    | `TestLookupMissingArgs` in `tests/mock-apps/parking_app_test.go` |

TS-09-E1 covers 09-REQ-1.E1 (location-sensor missing args), not
09-REQ-4.E1 (parking-app-cli missing flags).  No dedicated TS-09-Exx test
spec entry exists for parking-app-cli argument validation.
`TestLookupMissingArgs` and `TestInstallMissingArgs` provide the actual
coverage.

## Misalignment 2: 09-REQ-7.E3

| Field       | Matrix says          | Actual                           |
|-------------|----------------------|----------------------------------|
| Requirement | 09-REQ-7.E3          | companion-app-cli non-2xx        |
| Mapped to   | TS-09-E11            | PARKING_FEE_SERVICE Non-2xx      |
| Real test   | —                    | `TestCompanionHTTPError` in `tests/mock-apps/companion_test.go` |

TS-09-E11 covers parking-app-cli non-2xx from PARKING_FEE_SERVICE, not
companion-app-cli non-2xx from CLOUD_GATEWAY.  `TestCompanionHTTPError`
provides the actual coverage for 09-REQ-7.E3.

## Resolution

No code change needed.  Both requirements have automated test coverage via
the correct test functions listed above.  A future spec revision could add
dedicated test spec entries (e.g., TS-09-E12 for 09-REQ-4.E1, TS-09-E13
for 09-REQ-7.E3) or correct the matrix mappings.
