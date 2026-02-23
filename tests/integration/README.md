# Integration Tests

This directory contains cross-component integration tests. Integration tests
require local infrastructure services (Mosquitto, Kuksa Databroker) to be
running. Use `make infra-up` before running integration tests.

Tests in this directory are separate from component-level unit tests and verify
end-to-end behavior across service boundaries.
