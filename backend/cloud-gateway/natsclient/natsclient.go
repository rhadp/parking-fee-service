package natsclient

import (
	"errors"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// NATSClient wraps a NATS connection for the cloud-gateway.
type NATSClient struct{}

// Connect establishes a connection to the NATS server with exponential backoff retry.
func Connect(url string, maxRetries int) (*NATSClient, error) {
	return nil, errors.New("not implemented")
}

// PublishCommand publishes a command to NATS with the bearer token as a header.
func (nc *NATSClient) PublishCommand(vin string, cmd model.Command, token string) error {
	return errors.New("not implemented")
}

// SubscribeResponses subscribes to vehicle command responses and stores them.
func (nc *NATSClient) SubscribeResponses(s *store.Store) error {
	return errors.New("not implemented")
}

// SubscribeTelemetry subscribes to vehicle telemetry and logs it.
func (nc *NATSClient) SubscribeTelemetry() error {
	return errors.New("not implemented")
}

// Drain drains the NATS connection for graceful shutdown.
func (nc *NATSClient) Drain() error {
	return errors.New("not implemented")
}
