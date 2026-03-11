package main

import "github.com/nats-io/nats.go"

// NATSClient wraps a NATS connection for command publishing and subscription.
type NATSClient struct {
	conn *nats.Conn
}

// NewNATSClient creates a new NATS client connected to the given URL.
func NewNATSClient(url string) (*NATSClient, error) {
	// Stub - to be implemented
	return nil, nil
}

// Close closes the NATS connection.
func (nc *NATSClient) Close() {
	// Stub - to be implemented
}

// IsConnected returns true if the NATS connection is active.
func (nc *NATSClient) IsConnected() bool {
	// Stub - to be implemented
	return false
}

// PublishCommand publishes a command to the NATS subject for the given VIN.
func (nc *NATSClient) PublishCommand(vin string, cmd NATSCommand) error {
	// Stub - to be implemented
	return nil
}

// SubscribeCommandResponses subscribes to command responses for the given VIN.
func (nc *NATSClient) SubscribeCommandResponses(vin string, handler func(NATSCommandResponse)) error {
	// Stub - to be implemented
	return nil
}

// SubscribeTelemetry subscribes to telemetry for the given VIN.
func (nc *NATSClient) SubscribeTelemetry(vin string, handler func(TelemetryData)) error {
	// Stub - to be implemented
	return nil
}
