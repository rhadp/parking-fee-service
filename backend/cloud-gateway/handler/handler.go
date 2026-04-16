package handler

import (
	"net/http"
	"time"

	"parking-fee-service/backend/cloud-gateway/model"
	"parking-fee-service/backend/cloud-gateway/store"
)

// NATSPublisher is the interface used by command handler to publish to NATS.
type NATSPublisher interface {
	PublishCommand(vin string, cmd model.Command, token string) error
}

// NewSubmitCommandHandler returns an HTTP handler for POST /vehicles/{vin}/commands.
func NewSubmitCommandHandler(nc NATSPublisher, s *store.Store, timeout time.Duration) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, `{"error":"not implemented"}`, http.StatusInternalServerError)
	}
}

// NewGetCommandStatusHandler returns an HTTP handler for GET /vehicles/{vin}/commands/{command_id}.
func NewGetCommandStatusHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, `{"error":"not implemented"}`, http.StatusInternalServerError)
	}
}

// HealthHandler returns an HTTP handler for GET /health.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, `{"error":"not implemented"}`, http.StatusInternalServerError)
	}
}
