// Package middleware provides HTTP middleware and utilities for the parking-fee-service.
package middleware

import (
	"encoding/json"
	"net/http"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
)

// WriteError writes a JSON error response with the specified status code, error code, and message.
// The request ID is automatically included from the request context.
func WriteError(w http.ResponseWriter, r *http.Request, status int, code, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)

	response := model.ErrorResponse{
		Error:     code,
		Message:   message,
		RequestID: GetRequestID(r.Context()),
	}

	_ = json.NewEncoder(w).Encode(response)
}

// WriteValidationError writes a 400 Bad Request error for validation failures.
func WriteValidationError(w http.ResponseWriter, r *http.Request, message string) {
	WriteError(w, r, http.StatusBadRequest, model.ErrValidationError, message)
}

// WriteInvalidParameters writes a 400 Bad Request error for invalid parameters.
func WriteInvalidParameters(w http.ResponseWriter, r *http.Request, message string) {
	WriteError(w, r, http.StatusBadRequest, model.ErrInvalidParameters, message)
}

// WriteNotFound writes a 404 Not Found error with the specified error code.
func WriteNotFound(w http.ResponseWriter, r *http.Request, code, message string) {
	WriteError(w, r, http.StatusNotFound, code, message)
}

// WriteZoneNotFound writes a 404 Not Found error for zone lookups.
func WriteZoneNotFound(w http.ResponseWriter, r *http.Request) {
	WriteNotFound(w, r, model.ErrZoneNotFound, "No parking zone found for location")
}

// WriteAdapterNotFound writes a 404 Not Found error for adapter lookups.
func WriteAdapterNotFound(w http.ResponseWriter, r *http.Request, adapterID string) {
	WriteNotFound(w, r, model.ErrAdapterNotFound, "Adapter not found: "+adapterID)
}

// WriteSessionNotFound writes a 404 Not Found error for session lookups.
func WriteSessionNotFound(w http.ResponseWriter, r *http.Request, sessionID string) {
	WriteNotFound(w, r, model.ErrSessionNotFound, "Session not found: "+sessionID)
}

// WriteDatabaseError writes a 500 Internal Server Error for database failures.
func WriteDatabaseError(w http.ResponseWriter, r *http.Request, message string) {
	WriteError(w, r, http.StatusInternalServerError, model.ErrDatabaseError, message)
}

// WriteInternalError writes a 500 Internal Server Error for unexpected failures.
func WriteInternalError(w http.ResponseWriter, r *http.Request, message string) {
	WriteError(w, r, http.StatusInternalServerError, model.ErrInternalError, message)
}

// WriteJSON writes a JSON response with the specified status code.
func WriteJSON(w http.ResponseWriter, status int, data interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(data)
}
