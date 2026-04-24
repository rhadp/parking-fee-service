package natsclient

import (
	"github.com/nats-io/nats.go"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/store"
)

// NATSClient wraps a NATS connection for command publishing and response subscription.
type NATSClient struct {
	conn *nats.Conn
}

// Connect establishes a NATS connection with exponential backoff retry.
func Connect(url string, maxRetries int) (*NATSClient, error) {
	return nil, nil
}

// PublishCommand publishes a command to the NATS subject for the given VIN.
func (nc *NATSClient) PublishCommand(vin string, cmd model.Command, token string) error {
	return nil
}

// SubscribeResponses subscribes to command responses from all vehicles.
func (nc *NATSClient) SubscribeResponses(s *store.Store) error {
	return nil
}

// SubscribeTelemetry subscribes to telemetry from all vehicles.
func (nc *NATSClient) SubscribeTelemetry() error {
	return nil
}

// Drain gracefully drains the NATS connection.
func (nc *NATSClient) Drain() error {
	return nil
}
