// Package natsclient manages NATS connections, publishing commands,
// and subscribing to responses and telemetry.
package natsclient

import (
	"github.com/nats-io/nats.go"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// NATSClient wraps a NATS connection for publishing commands and
// subscribing to responses and telemetry.
type NATSClient struct {
	conn *nats.Conn
}

// Connect establishes a connection to the NATS server at the given URL,
// retrying with exponential backoff (1s, 2s, 4s, 8s) up to maxRetries
// attempts. Returns an error if all attempts fail.
func Connect(url string, maxRetries int) (*NATSClient, error) {
	// TODO: implement
	return nil, nil
}

// PublishCommand publishes a command to the NATS subject vehicles.{vin}.commands
// with the bearer token in the Authorization header.
func (nc *NATSClient) PublishCommand(vin string, cmd model.Command, token string) error {
	// TODO: implement
	return nil
}

// SubscribeResponses subscribes to vehicles.*.command_responses and stores
// incoming responses in the provided store.
func (nc *NATSClient) SubscribeResponses(s *store.Store) error {
	// TODO: implement
	return nil
}

// SubscribeTelemetry subscribes to vehicles.*.telemetry and logs incoming data.
func (nc *NATSClient) SubscribeTelemetry() error {
	// TODO: implement
	return nil
}

// Drain drains the NATS connection for graceful shutdown.
func (nc *NATSClient) Drain() error {
	// TODO: implement
	return nil
}
