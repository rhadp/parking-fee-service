// Package model provides data structures for the cloud-gateway service.
package model

import "time"

// Command status constants
const (
	CommandStatusPending = "pending"
	CommandStatusSuccess = "success"
	CommandStatusFailed  = "failed"
	CommandStatusTimeout = "timeout"
)

// Command type constants
const (
	CommandTypeLock   = "lock"
	CommandTypeUnlock = "unlock"
)

// Door constants
const (
	DoorDriver = "driver"
	DoorAll    = "all"
)

// Error code constants
const (
	ErrInvalidCommandType = "INVALID_COMMAND_TYPE"
	ErrInvalidDoor        = "INVALID_DOOR"
	ErrMissingAuthToken   = "MISSING_AUTH_TOKEN"
	ErrVehicleNotFound    = "VEHICLE_NOT_FOUND"
	ErrCommandNotFound    = "COMMAND_NOT_FOUND"
	ErrNoActiveSession    = "NO_ACTIVE_SESSION"
	ErrTimeout            = "TIMEOUT"
	ErrInternalError      = "INTERNAL_ERROR"
)

// Audit event type constants
const (
	AuditEventCommandSubmission   = "command_submission"
	AuditEventCommandStatusChange = "command_status_change"
	AuditEventAuthAttempt         = "auth_attempt"
	AuditEventTelemetryUpdate     = "telemetry_update"
	AuditEventMQTTConnect         = "mqtt_connect"
	AuditEventMQTTDisconnect      = "mqtt_disconnect"
	AuditEventMQTTReconnect       = "mqtt_reconnect"
	AuditEventValidationFailure   = "validation_failure"
)

// Command represents a lock/unlock command.
type Command struct {
	CommandID    string     `json:"command_id"`
	CommandType  string     `json:"command_type"`
	Doors        []string   `json:"doors"`
	AuthToken    string     `json:"-"` // not serialized in responses
	Status       string     `json:"status"`
	CreatedAt    time.Time  `json:"created_at"`
	CompletedAt  *time.Time `json:"completed_at,omitempty"`
	ErrorCode    string     `json:"error_code,omitempty"`
	ErrorMessage string     `json:"error_message,omitempty"`
}

// Telemetry represents vehicle telemetry data.
type Telemetry struct {
	Timestamp            time.Time `json:"timestamp"`
	Latitude             float64   `json:"latitude"`
	Longitude            float64   `json:"longitude"`
	DoorLocked           bool      `json:"door_locked"`
	DoorOpen             bool      `json:"door_open"`
	ParkingSessionActive bool      `json:"parking_session_active"`
	ReceivedAt           time.Time `json:"received_at"`
}

// ParkingSession represents parking session data from PARKING_FEE_SERVICE.
type ParkingSession struct {
	SessionID       string  `json:"session_id"`
	ZoneName        string  `json:"zone_name"`
	HourlyRate      float64 `json:"hourly_rate"`
	Currency        string  `json:"currency"`
	DurationSeconds int64   `json:"duration_seconds"`
	CurrentCost     float64 `json:"current_cost"`
	Timestamp       string  `json:"timestamp"`
}

// SubmitCommandRequest is the request body for command submission.
type SubmitCommandRequest struct {
	CommandType string   `json:"command_type"`
	Doors       []string `json:"doors"`
	AuthToken   string   `json:"auth_token"`
}

// SubmitCommandResponse is the response for command submission.
type SubmitCommandResponse struct {
	CommandID string `json:"command_id"`
	Status    string `json:"status"`
	RequestID string `json:"request_id"`
}

// CommandStatusResponse is the response for command status query.
type CommandStatusResponse struct {
	CommandID    string  `json:"command_id"`
	CommandType  string  `json:"command_type"`
	Status       string  `json:"status"`
	CreatedAt    string  `json:"created_at"`
	CompletedAt  *string `json:"completed_at,omitempty"`
	ErrorCode    string  `json:"error_code,omitempty"`
	ErrorMessage string  `json:"error_message,omitempty"`
	RequestID    string  `json:"request_id"`
}

// ParkingSessionResponse is the response for parking session query.
type ParkingSessionResponse struct {
	SessionID       string  `json:"session_id"`
	ZoneName        string  `json:"zone_name"`
	HourlyRate      float64 `json:"hourly_rate"`
	Currency        string  `json:"currency"`
	DurationSeconds int64   `json:"duration_seconds"`
	CurrentCost     float64 `json:"current_cost"`
	Timestamp       string  `json:"timestamp"`
	RequestID       string  `json:"request_id"`
}

// ErrorResponse is the standard error response format.
type ErrorResponse struct {
	ErrorCode string `json:"error_code"`
	Message   string `json:"message"`
	RequestID string `json:"request_id"`
}

// HealthResponse is the response for health check.
type HealthResponse struct {
	Status    string `json:"status"`
	Service   string `json:"service"`
	Timestamp string `json:"timestamp"`
}

// ReadyResponse is the response for readiness check.
type ReadyResponse struct {
	Status        string `json:"status"`
	MQTTConnected bool   `json:"mqtt_connected"`
}

// MQTTCommandMessage is the message published to vehicle via MQTT.
type MQTTCommandMessage struct {
	CommandID string   `json:"command_id"`
	Type      string   `json:"type"`
	Doors     []string `json:"doors"`
	AuthToken string   `json:"auth_token"`
	Timestamp string   `json:"timestamp"`
}

// MQTTCommandResponse is the command response from vehicle via MQTT.
type MQTTCommandResponse struct {
	CommandID    string `json:"command_id"`
	Status       string `json:"status"`
	ErrorCode    string `json:"error_code,omitempty"`
	ErrorMessage string `json:"error_message,omitempty"`
	Timestamp    string `json:"timestamp"`
}

// MQTTTelemetryMessage is the telemetry message from vehicle via MQTT.
type MQTTTelemetryMessage struct {
	Timestamp            string  `json:"timestamp"`
	Latitude             float64 `json:"latitude"`
	Longitude            float64 `json:"longitude"`
	DoorLocked           bool    `json:"door_locked"`
	DoorOpen             bool    `json:"door_open"`
	ParkingSessionActive bool    `json:"parking_session_active"`
}

// AuditEventBase contains base fields for all audit log entries.
type AuditEventBase struct {
	LogType       string    `json:"log_type"`       // Always "audit"
	CorrelationID string    `json:"correlation_id"` // Request tracing ID
	Timestamp     time.Time `json:"timestamp"`
}

// NewAuditEventBase creates a new AuditEventBase with common fields.
func NewAuditEventBase(correlationID string) AuditEventBase {
	return AuditEventBase{
		LogType:       "audit",
		CorrelationID: correlationID,
		Timestamp:     time.Now(),
	}
}

// CommandSubmissionEvent logs command submission details.
type CommandSubmissionEvent struct {
	AuditEventBase
	EventType   string   `json:"event_type"`
	VIN         string   `json:"vin"`
	CommandType string   `json:"command_type"`
	Doors       []string `json:"doors"`
	SourceIP    string   `json:"source_ip"`
	RequestID   string   `json:"request_id"`
	CommandID   string   `json:"command_id"`
}

// CommandStatusChangeEvent logs command status transitions.
type CommandStatusChangeEvent struct {
	AuditEventBase
	EventType      string `json:"event_type"`
	CommandID      string `json:"command_id"`
	PreviousStatus string `json:"previous_status"`
	NewStatus      string `json:"new_status"`
}

// AuthAttemptEvent logs authentication attempts.
type AuthAttemptEvent struct {
	AuditEventBase
	EventType     string `json:"event_type"`
	VIN           string `json:"vin"`
	AuthTokenHash string `json:"auth_token_hash"` // First 8 chars of SHA256 hash
	Success       bool   `json:"success"`
	SourceIP      string `json:"source_ip"`
}

// TelemetryUpdateEvent logs telemetry updates.
type TelemetryUpdateEvent struct {
	AuditEventBase
	EventType        string `json:"event_type"`
	VIN              string `json:"vin"`
	LocationPresent  bool   `json:"location_present"`
	DoorStateChanged bool   `json:"door_state_changed"`
}

// MQTTConnectionEvent logs MQTT connection events.
type MQTTConnectionEvent struct {
	AuditEventBase
	EventType     string `json:"event_type"` // mqtt_connect, mqtt_disconnect, mqtt_reconnect
	BrokerAddress string `json:"broker_address"`
}

// ValidationFailureEvent logs validation failures.
type ValidationFailureEvent struct {
	AuditEventBase
	EventType       string `json:"event_type"`
	VIN             string `json:"vin"`
	Endpoint        string `json:"endpoint"`
	ValidationError string `json:"validation_error"`
	SourceIP        string `json:"source_ip"`
}

// ValidationError represents a validation error.
type ValidationError struct {
	Code    string
	Message string
}

// Error implements the error interface.
func (e *ValidationError) Error() string {
	return e.Message
}
