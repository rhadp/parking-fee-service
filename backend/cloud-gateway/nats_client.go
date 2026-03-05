package main

import (
	"encoding/json"
	"fmt"
	"log"

	"github.com/nats-io/nats.go"
)

// NATSClient wraps a NATS connection for publishing commands and subscribing to responses.
type NATSClient struct {
	conn *nats.Conn
}

// NewNATSClient creates a new NATSClient connected to the given URL.
func NewNATSClient(url string) (*NATSClient, error) {
	conn, err := nats.Connect(url)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to NATS at %s: %w", url, err)
	}
	return &NATSClient{conn: conn}, nil
}

// Close closes the NATS connection.
func (nc *NATSClient) Close() {
	if nc.conn != nil {
		nc.conn.Close()
	}
}

// IsConnected returns true if the NATS connection is active.
func (nc *NATSClient) IsConnected() bool {
	if nc.conn == nil {
		return false
	}
	return nc.conn.IsConnected()
}

// PublishCommand publishes a command to the NATS subject for the given VIN.
// The subject follows the pattern: vehicles.{vin}.commands
func (nc *NATSClient) PublishCommand(vin string, cmd NATSCommand) error {
	if nc.conn == nil || !nc.conn.IsConnected() {
		return fmt.Errorf("NATS connection is not available")
	}

	data, err := json.Marshal(cmd)
	if err != nil {
		return fmt.Errorf("failed to marshal command: %w", err)
	}

	subject := "vehicles." + vin + ".commands"
	if err := nc.conn.Publish(subject, data); err != nil {
		return fmt.Errorf("failed to publish to %s: %w", subject, err)
	}

	return nc.conn.Flush()
}

// SubscribeCommandResponses subscribes to command responses for the given VIN.
// The subject follows the pattern: vehicles.{vin}.command_responses
func (nc *NATSClient) SubscribeCommandResponses(vin string, handler func(NATSCommandResponse)) error {
	if nc.conn == nil {
		return fmt.Errorf("NATS connection is not available")
	}

	subject := "vehicles." + vin + ".command_responses"
	_, err := nc.conn.Subscribe(subject, func(msg *nats.Msg) {
		var resp NATSCommandResponse
		if err := json.Unmarshal(msg.Data, &resp); err != nil {
			log.Printf("failed to unmarshal command response on %s: %v", subject, err)
			return
		}
		handler(resp)
	})
	if err != nil {
		return fmt.Errorf("failed to subscribe to %s: %w", subject, err)
	}

	return nil
}

// SubscribeTelemetry subscribes to telemetry data for the given VIN.
// The subject follows the pattern: vehicles.{vin}.telemetry
// Invalid JSON messages are logged and discarded.
func (nc *NATSClient) SubscribeTelemetry(vin string, handler func(TelemetryData)) error {
	if nc.conn == nil {
		return fmt.Errorf("NATS connection is not available")
	}

	subject := "vehicles." + vin + ".telemetry"
	_, err := nc.conn.Subscribe(subject, func(msg *nats.Msg) {
		var data TelemetryData
		if err := json.Unmarshal(msg.Data, &data); err != nil {
			log.Printf("failed to unmarshal telemetry on %s: %v (discarding)", subject, err)
			return
		}
		handler(data)
	})
	if err != nil {
		return fmt.Errorf("failed to subscribe to %s: %w", subject, err)
	}

	return nil
}
