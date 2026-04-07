package handler

import (
	"net/http"
	"time"

	"parking-fee-service/backend/cloud-gateway/model"
	"parking-fee-service/backend/cloud-gateway/store"
)

// NATSPublisher is the interface used by the handler to publish commands to NATS.
type NATSPublisher interface {
	PublishCommand(vin string, cmd model.Command, token string) error
}

// NewSubmitCommandHandler returns an HTTP handler for POST /vehicles/{vin}/commands.
func NewSubmitCommandHandler(pub NATSPublisher, s *store.Store, timeout time.Duration) http.HandlerFunc {
	panic("not implemented")
}

// NewGetCommandStatusHandler returns an HTTP handler for GET /vehicles/{vin}/commands/{command_id}.
func NewGetCommandStatusHandler(s *store.Store) http.HandlerFunc {
	panic("not implemented")
}

// HealthHandler returns an HTTP handler for GET /health.
func HealthHandler() http.HandlerFunc {
	panic("not implemented")
}
