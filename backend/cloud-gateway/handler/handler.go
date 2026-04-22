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

// NewSubmitCommandHandler returns a handler that accepts command submissions,
// publishes them via the CommandPublisher, and starts a timeout timer.
func NewSubmitCommandHandler(pub CommandPublisher, s *store.Store, timeout time.Duration) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// stub
	}
}

// NewGetCommandStatusHandler returns a handler that looks up and returns
// the status of a previously submitted command.
func NewGetCommandStatusHandler(s *store.Store) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// stub
	}
}

// HealthHandler returns a handler that responds with a health check status.
func HealthHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// stub
	}
}
