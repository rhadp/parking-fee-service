// Package handler provides HTTP request handlers for the cloud-gateway
// REST API.
package handler

import (
	"net/http"
	"time"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// Commander is an interface for publishing commands to NATS.
// This allows handler tests to use a mock instead of a real NATS connection.
type Commander interface {
	PublishCommand(vin string, cmd any, token string) error
}

// NewSubmitCommandHandler returns an HTTP handler for POST /vehicles/{vin}/commands.
// It parses the command, validates it, publishes to NATS, and starts a timeout.
func NewSubmitCommandHandler(nc Commander, s *store.Store, timeout time.Duration) http.HandlerFunc {
	// TODO: implement
	return func(w http.ResponseWriter, r *http.Request) {}
}

// NewGetCommandStatusHandler returns an HTTP handler for GET /vehicles/{vin}/commands/{command_id}.
// It looks up the command response in the store.
func NewGetCommandStatusHandler(s *store.Store) http.HandlerFunc {
	// TODO: implement
	return func(w http.ResponseWriter, r *http.Request) {}
}

// HealthHandler returns an HTTP handler for GET /health.
func HealthHandler() http.HandlerFunc {
	// TODO: implement
	return func(w http.ResponseWriter, r *http.Request) {}
}
