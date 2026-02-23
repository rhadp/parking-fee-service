// Package mqtt provides an MQTT client wrapper for the CLOUD_GATEWAY service.
// It handles connection management, publish/subscribe operations, and automatic
// reconnection with exponential backoff.
package mqtt

import "fmt"

// CommandTopic returns the MQTT topic for sending commands to a specific vehicle.
// Format: vehicles/{vin}/commands
func CommandTopic(vin string) string {
	return fmt.Sprintf("vehicles/%s/commands", vin)
}

// ResponseTopic returns the MQTT topic for receiving command responses from a
// specific vehicle.
// Format: vehicles/{vin}/command_responses
func ResponseTopic(vin string) string {
	return fmt.Sprintf("vehicles/%s/command_responses", vin)
}

// TelemetryTopic returns the MQTT topic for receiving telemetry data from a
// specific vehicle.
// Format: vehicles/{vin}/telemetry
func TelemetryTopic(vin string) string {
	return fmt.Sprintf("vehicles/%s/telemetry", vin)
}

// WildcardResponseTopic returns the wildcard MQTT topic for subscribing to
// command responses from all vehicles.
// Format: vehicles/+/command_responses
func WildcardResponseTopic() string {
	return "vehicles/+/command_responses"
}

// WildcardTelemetryTopic returns the wildcard MQTT topic for subscribing to
// telemetry data from all vehicles.
// Format: vehicles/+/telemetry
func WildcardTelemetryTopic() string {
	return "vehicles/+/telemetry"
}

// ExtractVINFromTopic extracts the VIN from an MQTT topic string.
// It expects topics in the format "vehicles/{vin}/{suffix}".
// Returns the VIN and true if extraction succeeds, or empty string and false otherwise.
func ExtractVINFromTopic(topic string) (string, bool) {
	// Find first slash after "vehicles/"
	const prefix = "vehicles/"
	if len(topic) <= len(prefix) {
		return "", false
	}
	if topic[:len(prefix)] != prefix {
		return "", false
	}

	rest := topic[len(prefix):]
	// Find the next slash
	for i := 0; i < len(rest); i++ {
		if rest[i] == '/' {
			if i == 0 {
				return "", false
			}
			return rest[:i], true
		}
	}
	return "", false
}
