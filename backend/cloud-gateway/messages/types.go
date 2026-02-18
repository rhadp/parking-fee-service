// Package messages defines the shared MQTT message types and topic patterns
// used by CLOUD_GATEWAY and CLOUD_GATEWAY_CLIENT for vehicle-to-cloud
// communication.
//
// All message types are serialized as JSON over MQTT. The schemas defined here
// must match the Rust-side definitions in cloud-gateway-client/src/messages.rs
// to ensure wire-format compatibility.
package messages

import "fmt"

// MQTT topic patterns. The placeholder {vin} is replaced with the actual VIN.
const (
	// TopicCommands is published by CLOUD_GATEWAY to send lock/unlock
	// commands to a specific vehicle (QoS 2).
	TopicCommands = "vehicles/%s/commands"

	// TopicCommandResponses is published by CLOUD_GATEWAY_CLIENT to report
	// command results back to CLOUD_GATEWAY (QoS 2).
	TopicCommandResponses = "vehicles/%s/command_responses"

	// TopicStatusRequest is published by CLOUD_GATEWAY to request on-demand
	// vehicle status (QoS 2).
	TopicStatusRequest = "vehicles/%s/status_request"

	// TopicStatusResponse is published by CLOUD_GATEWAY_CLIENT in response
	// to a status request (QoS 2).
	TopicStatusResponse = "vehicles/%s/status_response"

	// TopicTelemetry is published periodically by CLOUD_GATEWAY_CLIENT with
	// current vehicle state (QoS 0).
	TopicTelemetry = "vehicles/%s/telemetry"

	// TopicRegistration is published by CLOUD_GATEWAY_CLIENT on startup to
	// register the vehicle with CLOUD_GATEWAY (QoS 2).
	TopicRegistration = "vehicles/%s/registration"

	// Wildcard subscription patterns used by CLOUD_GATEWAY.

	// SubCommandResponses subscribes to all vehicle command responses.
	SubCommandResponses = "vehicles/+/command_responses"

	// SubStatusResponse subscribes to all vehicle status responses.
	SubStatusResponse = "vehicles/+/status_response"

	// SubTelemetry subscribes to all vehicle telemetry messages.
	SubTelemetry = "vehicles/+/telemetry"

	// SubRegistration subscribes to all vehicle registration messages.
	SubRegistration = "vehicles/+/registration"
)

// CommandType represents the type of a lock/unlock command.
type CommandType string

const (
	// CommandTypeLock requests the vehicle to lock.
	CommandTypeLock CommandType = "lock"

	// CommandTypeUnlock requests the vehicle to unlock.
	CommandTypeUnlock CommandType = "unlock"
)

// CommandResult represents the outcome of a lock/unlock command.
type CommandResult string

const (
	// CommandResultSuccess indicates the command was executed successfully.
	CommandResultSuccess CommandResult = "SUCCESS"

	// CommandResultRejectedSpeed indicates the command was rejected because
	// the vehicle speed exceeded the safety threshold.
	CommandResultRejectedSpeed CommandResult = "REJECTED_SPEED"

	// CommandResultRejectedDoorOpen indicates the command was rejected
	// because a vehicle door was open.
	CommandResultRejectedDoorOpen CommandResult = "REJECTED_DOOR_OPEN"
)

// CommandMessage is published by CLOUD_GATEWAY to vehicles/{vin}/commands.
// It instructs the vehicle to lock or unlock.
type CommandMessage struct {
	CommandID string      `json:"command_id"`
	Type      CommandType `json:"type"`
	Timestamp int64       `json:"timestamp"`
}

// CommandResponse is published by CLOUD_GATEWAY_CLIENT to
// vehicles/{vin}/command_responses. It reports the result of a command.
type CommandResponse struct {
	CommandID string        `json:"command_id"`
	Type      CommandType   `json:"type"`
	Result    CommandResult `json:"result"`
	Timestamp int64         `json:"timestamp"`
}

// StatusRequest is published by CLOUD_GATEWAY to
// vehicles/{vin}/status_request. It requests the current vehicle state.
type StatusRequest struct {
	RequestID string `json:"request_id"`
	Timestamp int64  `json:"timestamp"`
}

// StatusResponse is published by CLOUD_GATEWAY_CLIENT to
// vehicles/{vin}/status_response in reply to a StatusRequest.
type StatusResponse struct {
	RequestID            string   `json:"request_id"`
	VIN                  string   `json:"vin"`
	IsLocked             *bool    `json:"is_locked"`
	IsDoorOpen           *bool    `json:"is_door_open"`
	Speed                *float64 `json:"speed"`
	Latitude             *float64 `json:"latitude"`
	Longitude            *float64 `json:"longitude"`
	ParkingSessionActive *bool    `json:"parking_session_active"`
	Timestamp            int64    `json:"timestamp"`
}

// TelemetryMessage is published periodically by CLOUD_GATEWAY_CLIENT to
// vehicles/{vin}/telemetry with the current vehicle state.
type TelemetryMessage struct {
	VIN                  string   `json:"vin"`
	IsLocked             *bool    `json:"is_locked"`
	IsDoorOpen           *bool    `json:"is_door_open"`
	Speed                *float64 `json:"speed"`
	Latitude             *float64 `json:"latitude"`
	Longitude            *float64 `json:"longitude"`
	ParkingSessionActive *bool    `json:"parking_session_active"`
	Timestamp            int64    `json:"timestamp"`
}

// RegistrationMessage is published by CLOUD_GATEWAY_CLIENT to
// vehicles/{vin}/registration on startup to register the vehicle.
type RegistrationMessage struct {
	VIN        string `json:"vin"`
	PairingPIN string `json:"pairing_pin"`
	Timestamp  int64  `json:"timestamp"`
}

// TopicFor returns a fully-qualified MQTT topic by replacing the %s placeholder
// with the given VIN.
func TopicFor(pattern, vin string) string {
	return fmt.Sprintf(pattern, vin)
}
