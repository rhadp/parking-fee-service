// Package natsclient provides NATS connectivity for the CLOUD_GATEWAY.
package natsclient

import (
	"fmt"

	nats "github.com/nats-io/nats.go"

	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/model"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/store"
)

// NATSClient wraps a NATS connection and exposes domain-level publish/subscribe
// operations for the CLOUD_GATEWAY.
type NATSClient struct {
	conn *nats.Conn
}

// Connect establishes a connection to the NATS server at url, retrying up to
// maxRetries times with exponential backoff (1s, 2s, 4s, ...).
// Returns an error if all attempts fail.
// STUB: returns an error immediately (not implemented).
func Connect(url string, maxRetries int) (*NATSClient, error) {
	return nil, fmt.Errorf("not implemented: Connect(%q, %d)", url, maxRetries)
}

// PublishCommand publishes cmd to the NATS subject vehicles.{vin}.commands,
// setting an Authorization header on the NATS message with the provided token.
// STUB: returns an error (not implemented).
func (nc *NATSClient) PublishCommand(vin string, cmd model.Command, token string) error {
	return fmt.Errorf("not implemented: PublishCommand")
}

// SubscribeResponses subscribes to vehicles.*.command_responses, parses each
// incoming JSON message as a CommandResponse, and stores it via s.StoreResponse.
// STUB: returns an error (not implemented).
func (nc *NATSClient) SubscribeResponses(s *store.Store) error {
	return fmt.Errorf("not implemented: SubscribeResponses")
}

// SubscribeTelemetry subscribes to vehicles.*.telemetry and logs each message.
// STUB: returns an error (not implemented).
func (nc *NATSClient) SubscribeTelemetry() error {
	return fmt.Errorf("not implemented: SubscribeTelemetry")
}

// Drain drains the underlying NATS connection for graceful shutdown.
// STUB: returns an error (not implemented).
func (nc *NATSClient) Drain() error {
	return fmt.Errorf("not implemented: Drain")
}
