package main

import "github.com/nats-io/nats.go"

// NATSClient wraps a NATS connection for publishing commands and subscribing to responses.
type NATSClient struct {
	conn *nats.Conn
}

// NewNATSClient creates a new NATSClient connected to the given URL.
func NewNATSClient(url string) (*NATSClient, error) {
	// Stub: not yet implemented
	return nil, nil
}

// Close closes the NATS connection.
func (nc *NATSClient) Close() {
	// Stub: not yet implemented
}

// IsConnected returns true if the NATS connection is active.
func (nc *NATSClient) IsConnected() bool {
	// Stub: not yet implemented
	return false
}

// PublishCommand publishes a command to the NATS subject for the given VIN.
func (nc *NATSClient) PublishCommand(vin string, cmd NATSCommand) error {
	// Stub: not yet implemented
	return nil
}

// SubscribeCommandResponses subscribes to command responses for the given VIN.
func (nc *NATSClient) SubscribeCommandResponses(vin string, handler func(NATSCommandResponse)) error {
	// Stub: not yet implemented
	return nil
}

// SubscribeTelemetry subscribes to telemetry data for the given VIN.
func (nc *NATSClient) SubscribeTelemetry(vin string, handler func(TelemetryData)) error {
	// Stub: not yet implemented
	return nil
}
