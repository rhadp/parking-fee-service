package main

// StartRequest represents the JSON body for POST /parking/start.
type StartRequest struct {
	VehicleID string `json:"vehicle_id"`
	ZoneID    string `json:"zone_id"`
	Timestamp int64  `json:"timestamp"`
}

// StartResponse represents the JSON response for POST /parking/start.
type StartResponse struct {
	SessionID string `json:"session_id"`
	Status    string `json:"status"`
}

// StopRequest represents the JSON body for POST /parking/stop.
type StopRequest struct {
	SessionID string `json:"session_id"`
}

// StopResponse represents the JSON response for POST /parking/stop.
type StopResponse struct {
	SessionID       string  `json:"session_id"`
	DurationSeconds int64   `json:"duration_seconds"`
	Fee             float64 `json:"fee"`
	Status          string  `json:"status"`
}

// ErrorResponse represents a JSON error response.
type ErrorResponse struct {
	Error string `json:"error"`
}
