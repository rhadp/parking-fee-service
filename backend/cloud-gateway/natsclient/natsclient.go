package natsclient

import (
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// NATSClient manages the NATS connection and provides methods for
// publishing commands and subscribing to responses and telemetry.
type NATSClient struct{}

// Connect establishes a connection to the NATS server at the given URL,
// retrying with exponential backoff up to maxRetries attempts.
func Connect(url string, maxRetries int) (*NATSClient, error) {
	return nil, nil // stub
}

// PublishCommand publishes a command to the NATS subject for the given VIN,
// including the bearer token as a message header.
func (nc *NATSClient) PublishCommand(vin string, cmd model.Command, token string) error {
	return nil // stub
}

// SubscribeResponses subscribes to command response messages from all vehicles
// and stores them in the provided Store.
func (nc *NATSClient) SubscribeResponses(s *store.Store) error {
	return nil // stub
}

// SubscribeTelemetry subscribes to telemetry messages from all vehicles
// and logs them.
func (nc *NATSClient) SubscribeTelemetry() error {
	return nil // stub
}

// Drain gracefully drains the NATS connection.
func (nc *NATSClient) Drain() error {
	return nil // stub
}
