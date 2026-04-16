package natsclient

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"strings"
	"time"

	nats "github.com/nats-io/nats.go"

	"parking-fee-service/backend/cloud-gateway/model"
	"parking-fee-service/backend/cloud-gateway/store"
)

// NATSClient wraps a NATS connection for command publishing and subscription.
type NATSClient struct {
	conn *nats.Conn
}

// Connect establishes a NATS connection with exponential backoff retry.
// maxRetries is the maximum number of connection attempts.
// Backoff delays: 1s, 2s, 4s (capped), repeated at 4s for additional attempts.
// Returns an error if all attempts fail.
func Connect(url string, maxRetries int) (*NATSClient, error) {
	delay := time.Second
	const maxDelay = 4 * time.Second

	for attempt := 1; attempt <= maxRetries; attempt++ {
		conn, err := nats.Connect(url)
		if err == nil {
			slog.Info("connected to NATS", "url", url, "attempt", attempt)
			return &NATSClient{conn: conn}, nil
		}
		slog.Warn("failed to connect to NATS", "url", url, "attempt", attempt, "error", err)

		if attempt < maxRetries {
			time.Sleep(delay)
			delay *= 2
			if delay > maxDelay {
				delay = maxDelay
			}
		}
	}

	return nil, fmt.Errorf("failed to connect to NATS at %s after %d attempts", url, maxRetries)
}

// PublishCommand publishes a command to vehicles.{vin}.commands with an Authorization header.
// Requirements: 06-REQ-1.2
func (nc *NATSClient) PublishCommand(vin string, cmd model.Command, token string) error {
	data, err := json.Marshal(cmd)
	if err != nil {
		return fmt.Errorf("failed to marshal command: %w", err)
	}

	subject := fmt.Sprintf("vehicles.%s.commands", vin)
	msg := &nats.Msg{
		Subject: subject,
		Header:  make(nats.Header),
		Data:    data,
	}
	msg.Header.Set("Authorization", "Bearer "+token)

	return nc.conn.PublishMsg(msg)
}

// SubscribeResponses subscribes to vehicles.*.command_responses and stores responses
// in the provided store.
// Requirements: 06-REQ-5.1, 06-REQ-5.2
func (nc *NATSClient) SubscribeResponses(s *store.Store) error {
	_, err := nc.conn.Subscribe("vehicles.*.command_responses", func(msg *nats.Msg) {
		var resp model.CommandResponse
		if err := json.Unmarshal(msg.Data, &resp); err != nil {
			slog.Error("failed to parse command response", "subject", msg.Subject, "error", err)
			return
		}
		slog.Debug("received command response", "command_id", resp.CommandID, "status", resp.Status)
		s.StoreResponse(resp)
	})
	return err
}

// SubscribeTelemetry subscribes to vehicles.*.telemetry and logs payloads.
// No storage or aggregation is performed.
// Requirements: 06-REQ-5.3
func (nc *NATSClient) SubscribeTelemetry() error {
	_, err := nc.conn.Subscribe("vehicles.*.telemetry", func(msg *nats.Msg) {
		// Extract VIN from subject (vehicles.{vin}.telemetry).
		parts := strings.Split(msg.Subject, ".")
		vin := ""
		if len(parts) == 3 {
			vin = parts[1]
		}
		slog.Info("telemetry received", "vin", vin, "data", string(msg.Data))
	})
	return err
}

// Drain drains the NATS connection for graceful shutdown.
// Requirements: 06-REQ-8.2
func (nc *NATSClient) Drain() error {
	return nc.conn.Drain()
}
