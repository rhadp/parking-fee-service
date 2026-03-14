// Package natsclient provides NATS client utilities for the cloud-gateway service.
// This is a stub package. Implementation will be added in task group 4.
package natsclient

// CommandSubject returns the NATS subject for publishing commands to a VIN.
func CommandSubject(vin string) string {
	return ""
}

// ResponseSubject returns the NATS wildcard subject for command responses.
func ResponseSubject() string {
	return ""
}

// TelemetrySubject returns the NATS wildcard subject for telemetry.
func TelemetrySubject() string {
	return ""
}
