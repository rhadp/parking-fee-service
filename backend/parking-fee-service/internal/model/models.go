// Package model defines the data models for the parking-fee-service.
package model

import "time"

// Error code constants for consistent error responses.
const (
	ErrInvalidParameters = "INVALID_PARAMETERS"
	ErrZoneNotFound      = "ZONE_NOT_FOUND"
	ErrAdapterNotFound   = "ADAPTER_NOT_FOUND"
	ErrSessionNotFound   = "SESSION_NOT_FOUND"
	ErrValidationError   = "VALIDATION_ERROR"
	ErrInternalError     = "INTERNAL_ERROR"
	ErrDatabaseError     = "DATABASE_ERROR"
)

// Session state constants.
const (
	SessionStateActive  = "active"
	SessionStateStopped = "stopped"
)

// Bounds represents a geographic bounding box for a zone.
type Bounds struct {
	MinLat float64 `json:"min_lat"`
	MaxLat float64 `json:"max_lat"`
	MinLng float64 `json:"min_lng"`
	MaxLng float64 `json:"max_lng"`
}

// ContainsPoint checks if the given coordinates are within the bounds.
func (b *Bounds) ContainsPoint(lat, lng float64) bool {
	return lat >= b.MinLat && lat <= b.MaxLat &&
		lng >= b.MinLng && lng <= b.MaxLng
}

// Zone represents a parking zone with associated operator and adapter information.
type Zone struct {
	ZoneID          string  `json:"zone_id"`
	OperatorName    string  `json:"operator_name"`
	HourlyRate      float64 `json:"hourly_rate"`
	Currency        string  `json:"currency"`
	AdapterImageRef string  `json:"adapter_image_ref"`
	AdapterChecksum string  `json:"adapter_checksum"`
	Bounds          Bounds  `json:"bounds"`
}

// Adapter represents a parking operator adapter with full details.
type Adapter struct {
	AdapterID    string    `json:"adapter_id"`
	OperatorName string    `json:"operator_name"`
	Version      string    `json:"version"`
	ImageRef     string    `json:"image_ref"`
	Checksum     string    `json:"checksum"`
	CreatedAt    time.Time `json:"created_at"`
}

// AdapterSummary represents a summary of an adapter for list responses.
type AdapterSummary struct {
	AdapterID    string `json:"adapter_id"`
	OperatorName string `json:"operator_name"`
	Version      string `json:"version"`
	ImageRef     string `json:"image_ref"`
}

// Session represents a parking session with all associated data.
type Session struct {
	SessionID     string     `json:"session_id"`
	VehicleID     string     `json:"vehicle_id"`
	ZoneID        string     `json:"zone_id"`
	Latitude      float64    `json:"latitude"`
	Longitude     float64    `json:"longitude"`
	StartTime     time.Time  `json:"start_time"`
	EndTime       *time.Time `json:"end_time,omitempty"`
	HourlyRate    float64    `json:"hourly_rate"`
	State         string     `json:"state"`
	TotalCost     *float64   `json:"total_cost,omitempty"`
	PaymentStatus *string    `json:"payment_status,omitempty"`
}

// SessionStatus represents the current status of a parking session.
type SessionStatus struct {
	SessionID       string  `json:"session_id"`
	State           string  `json:"state"`
	StartTime       string  `json:"start_time"`
	DurationSeconds int64   `json:"duration_seconds"`
	CurrentCost     float64 `json:"current_cost"`
	ZoneID          string  `json:"zone_id"`
}

// ZoneResponse is the response for zone lookup requests.
type ZoneResponse struct {
	ZoneID          string  `json:"zone_id"`
	OperatorName    string  `json:"operator_name"`
	HourlyRate      float64 `json:"hourly_rate"`
	Currency        string  `json:"currency"`
	AdapterImageRef string  `json:"adapter_image_ref"`
	AdapterChecksum string  `json:"adapter_checksum"`
}

// AdapterListResponse is the response for adapter list requests.
type AdapterListResponse struct {
	Adapters []AdapterSummary `json:"adapters"`
}

// StartSessionRequest is the request body for starting a parking session.
type StartSessionRequest struct {
	VehicleID string  `json:"vehicle_id"`
	Latitude  float64 `json:"latitude"`
	Longitude float64 `json:"longitude"`
	ZoneID    string  `json:"zone_id"`
	Timestamp string  `json:"timestamp"`
}

// StartSessionResponse is the response for starting a parking session.
type StartSessionResponse struct {
	SessionID  string  `json:"session_id"`
	ZoneID     string  `json:"zone_id"`
	HourlyRate float64 `json:"hourly_rate"`
	StartTime  string  `json:"start_time"`
}

// StopSessionRequest is the request body for stopping a parking session.
type StopSessionRequest struct {
	SessionID string `json:"session_id"`
	Timestamp string `json:"timestamp"`
}

// StopSessionResponse is the response for stopping a parking session.
type StopSessionResponse struct {
	SessionID       string  `json:"session_id"`
	StartTime       string  `json:"start_time"`
	EndTime         string  `json:"end_time"`
	DurationSeconds int64   `json:"duration_seconds"`
	TotalCost       float64 `json:"total_cost"`
	PaymentStatus   string  `json:"payment_status"`
}

// ErrorResponse is the standard error response format.
type ErrorResponse struct {
	Error     string `json:"error"`
	Message   string `json:"message"`
	RequestID string `json:"request_id"`
}

// HealthResponse is the response for health check requests.
type HealthResponse struct {
	Status    string `json:"status"`
	Service   string `json:"service"`
	Timestamp string `json:"timestamp"`
}

// ReadyResponse is the response for readiness check requests.
type ReadyResponse struct {
	Status string `json:"status"`
	Reason string `json:"reason,omitempty"`
}
