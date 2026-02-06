// Package handler provides HTTP handlers for the cloud-gateway service.
package handler

import (
	"encoding/json"
	"net/http"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/middleware"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
)

// WriteJSON writes a JSON response with the given status code.
func WriteJSON(w http.ResponseWriter, status int, data interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(data)
}

// WriteError writes an error response with the given status code, error code, and message.
func WriteError(w http.ResponseWriter, r *http.Request, status int, errorCode, message string) {
	requestID := middleware.GetRequestID(r.Context())
	response := model.ErrorResponse{
		ErrorCode: errorCode,
		Message:   message,
		RequestID: requestID,
	}
	WriteJSON(w, status, response)
}

// WriteValidationError writes a 400 Bad Request response for validation errors.
func WriteValidationError(w http.ResponseWriter, r *http.Request, errorCode, message string) {
	WriteError(w, r, http.StatusBadRequest, errorCode, message)
}

// WriteNotFound writes a 404 Not Found response.
func WriteNotFound(w http.ResponseWriter, r *http.Request, errorCode, message string) {
	WriteError(w, r, http.StatusNotFound, errorCode, message)
}

// WriteInternalError writes a 500 Internal Server Error response.
func WriteInternalError(w http.ResponseWriter, r *http.Request, message string) {
	WriteError(w, r, http.StatusInternalServerError, model.ErrInternalError, message)
}
