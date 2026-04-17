// Package handler provides HTTP handlers for the CLOUD_GATEWAY REST API.
package handler

import (
	"net/http"
	"time"

	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/model"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/store"
)

// NATSPublisher is the interface for publishing commands to NATS.
// Using an interface here lets handler tests inject a mock without requiring
// a real NATS server.
type NATSPublisher interface {
	PublishCommand(vin string, cmd model.Command, token string) error
}

// NewSubmitCommandHandler returns an HTTP handler for POST /vehicles/{vin}/commands.
// It parses the request body, validates the command, publishes to NATS, starts a
// timeout timer, and responds with HTTP 202.
// STUB: returns 501 Not Implemented.
func NewSubmitCommandHandler(nc NATSPublisher, s *store.Store, timeout time.Duration) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		http.Error(w, `{"error":"not implemented"}`, http.StatusNotImplemented)
	}
}

// NewGetCommandStatusHandler returns an HTTP handler for GET /vehicles/{vin}/commands/{command_id}.
// It looks up the command response in the store and returns it as JSON.
// STUB: returns 501 Not Implemented.
func NewGetCommandStatusHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		http.Error(w, `{"error":"not implemented"}`, http.StatusNotImplemented)
	}
}

// HealthHandler returns an HTTP handler for GET /health.
// It responds with HTTP 200 and {"status":"ok"}.
// STUB: returns 501 Not Implemented.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		http.Error(w, `{"error":"not implemented"}`, http.StatusNotImplemented)
	}
}
