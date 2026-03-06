package main

import "time"

// StartRequest is the JSON body for POST /parking/start.
type StartRequest struct {
	VehicleID string `json:"vehicle_id"`
	ZoneID    string `json:"zone_id"`
	Timestamp int64  `json:"timestamp"`
}

// StartResponse is returned by POST /parking/start.
type StartResponse struct {
	SessionID string `json:"session_id"`
	Status    string `json:"status"`
}

// StopRequest is the JSON body for POST /parking/stop.
type StopRequest struct {
	SessionID string `json:"session_id"`
}

// StopResponse is returned by POST /parking/stop.
type StopResponse struct {
	SessionID       string  `json:"session_id"`
	DurationSeconds int64   `json:"duration_seconds"`
	Fee             float64 `json:"fee"`
	Status          string  `json:"status"`
}

// Session represents an in-memory parking session record.
type Session struct {
	SessionID string    `json:"session_id"`
	VehicleID string    `json:"vehicle_id"`
	ZoneID    string    `json:"zone_id"`
	StartTime time.Time `json:"start_time"`
	Status    string    `json:"status"`
}

// ErrorResponse is a JSON error body.
type ErrorResponse struct {
	Error string `json:"error"`
}
