package natsclient

import (
	"errors"

	"parking-fee-service/backend/cloud-gateway/model"
	"parking-fee-service/backend/cloud-gateway/store"
)

var errNotImplemented = errors.New("not implemented")

// NATSClient wraps a NATS connection for command publishing and subscription.
type NATSClient struct{}

// Connect establishes a NATS connection with exponential backoff retry.
// maxRetries is the maximum number of connection attempts.
// Returns an error if all attempts fail.
func Connect(url string, maxRetries int) (*NATSClient, error) {
	return nil, errNotImplemented
}

// PublishCommand publishes a command to vehicles.{vin}.commands with Authorization header.
func (nc *NATSClient) PublishCommand(vin string, cmd model.Command, token string) error {
	return errNotImplemented
}

// SubscribeResponses subscribes to vehicles.*.command_responses and stores responses.
func (nc *NATSClient) SubscribeResponses(s *store.Store) error {
	return errNotImplemented
}

// SubscribeTelemetry subscribes to vehicles.*.telemetry and logs payloads.
func (nc *NATSClient) SubscribeTelemetry() error {
	return errNotImplemented
}

// Drain drains the NATS connection for graceful shutdown.
func (nc *NATSClient) Drain() error {
	return errNotImplemented
}
