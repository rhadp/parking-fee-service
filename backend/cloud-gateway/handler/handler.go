package handler

import (
	"net/http"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// CommandPublisher defines the interface for publishing commands to NATS.
type CommandPublisher interface {
	PublishCommand(vin string, cmd model.Command, token string) error
}

// NewSubmitCommandHandler returns a handler for POST /vehicles/{vin}/commands.
func NewSubmitCommandHandler(publisher CommandPublisher, s *store.Store, timeout time.Duration) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
	}
}

// NewGetCommandStatusHandler returns a handler for GET /vehicles/{vin}/commands/{command_id}.
func NewGetCommandStatusHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
	}
}

// HealthHandler returns a handler for GET /health.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
	}
}
