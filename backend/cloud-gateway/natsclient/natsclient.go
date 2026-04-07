package natsclient

import (
	"parking-fee-service/backend/cloud-gateway/model"
	"parking-fee-service/backend/cloud-gateway/store"

	"github.com/nats-io/nats.go"
)

// NATSClient wraps a NATS connection for the cloud gateway.
type NATSClient struct {
	conn *nats.Conn
}

// Connect establishes a connection to the NATS server with exponential backoff retry.
func Connect(url string, maxRetries int) (*NATSClient, error) {
	panic("not implemented")
}

// PublishCommand publishes a command to the NATS subject for the given VIN,
// including the bearer token as an Authorization header.
func (nc *NATSClient) PublishCommand(vin string, cmd model.Command, token string) error {
	panic("not implemented")
}

// SubscribeResponses subscribes to command responses and stores them.
func (nc *NATSClient) SubscribeResponses(s *store.Store) error {
	panic("not implemented")
}

// SubscribeTelemetry subscribes to telemetry messages and logs them.
func (nc *NATSClient) SubscribeTelemetry() error {
	panic("not implemented")
}

// Drain gracefully drains the NATS connection.
func (nc *NATSClient) Drain() error {
	panic("not implemented")
}
