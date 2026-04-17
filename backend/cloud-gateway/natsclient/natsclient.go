// Package natsclient provides NATS connectivity for the CLOUD_GATEWAY.
package natsclient

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"time"

	nats "github.com/nats-io/nats.go"

	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/model"
	"github.com/sdv-demo/parking-fee-service/backend/cloud-gateway/store"
)

// NATSClient wraps a NATS connection and exposes domain-level publish/subscribe
// operations for the CLOUD_GATEWAY.
type NATSClient struct {
	conn *nats.Conn
}

// backoffDelays defines the wait durations between consecutive connection attempts.
// With 5 total attempts, there are 4 inter-attempt delays: 1s, 2s, 4s, 8s.
var backoffDelays = []time.Duration{
	1 * time.Second,
	2 * time.Second,
	4 * time.Second,
	8 * time.Second,
}

// Connect establishes a connection to the NATS server at url, retrying up to
// maxRetries times with exponential backoff (1s, 2s, 4s, 8s between attempts).
// Returns an error if all attempts fail.
func Connect(url string, maxRetries int) (*NATSClient, error) {
	var lastErr error
	for attempt := 0; attempt < maxRetries; attempt++ {
		conn, err := nats.Connect(url)
		if err == nil {
			slog.Info("connected to NATS", "url", url, "attempt", attempt+1)
			return &NATSClient{conn: conn}, nil
		}
		lastErr = err
		slog.Warn("NATS connection failed", "url", url, "attempt", attempt+1, "error", err)

		// Wait before the next attempt (no wait after the last attempt).
		if attempt < maxRetries-1 && attempt < len(backoffDelays) {
			time.Sleep(backoffDelays[attempt])
		}
	}
	return nil, fmt.Errorf("failed to connect to NATS after %d attempts: %w", maxRetries, lastErr)
}

// PublishCommand publishes cmd to the NATS subject vehicles.{vin}.commands,
// setting an Authorization header on the NATS message with the provided token.
func (nc *NATSClient) PublishCommand(vin string, cmd model.Command, token string) error {
	payload, err := json.Marshal(cmd)
	if err != nil {
		return fmt.Errorf("marshal command: %w", err)
	}

	msg := &nats.Msg{
		Subject: fmt.Sprintf("vehicles.%s.commands", vin),
		Header:  nats.Header{},
		Data:    payload,
	}
	msg.Header.Set("Authorization", "Bearer "+token)

	if err := nc.conn.PublishMsg(msg); err != nil {
		return fmt.Errorf("publish command to NATS: %w", err)
	}
	return nil
}

// SubscribeResponses subscribes to vehicles.*.command_responses, parses each
// incoming JSON message as a CommandResponse, and stores it via s.StoreResponse.
func (nc *NATSClient) SubscribeResponses(s *store.Store) error {
	_, err := nc.conn.Subscribe("vehicles.*.command_responses", func(msg *nats.Msg) {
		var resp model.CommandResponse
		if err := json.Unmarshal(msg.Data, &resp); err != nil {
			slog.Warn("failed to parse command response from NATS",
				"subject", msg.Subject,
				"error", err,
				"payload", string(msg.Data))
			return
		}
		slog.Info("received command response", "command_id", resp.CommandID, "status", resp.Status)
		s.StoreResponse(resp)
	})
	if err != nil {
		return fmt.Errorf("subscribe to command responses: %w", err)
	}
	return nil
}

// SubscribeTelemetry subscribes to vehicles.*.telemetry and logs each message.
// No storage or aggregation is performed.
func (nc *NATSClient) SubscribeTelemetry() error {
	_, err := nc.conn.Subscribe("vehicles.*.telemetry", func(msg *nats.Msg) {
		// Extract VIN from subject: "vehicles.{vin}.telemetry"
		vin := extractVINFromSubject(msg.Subject)
		slog.Info("telemetry received",
			"vin", vin,
			"subject", msg.Subject,
			"payload", string(msg.Data))
	})
	if err != nil {
		return fmt.Errorf("subscribe to telemetry: %w", err)
	}
	return nil
}

// extractVINFromSubject extracts the VIN from a NATS subject of the form
// "vehicles.{vin}.telemetry". Returns an empty string if the subject does
// not match the expected pattern.
func extractVINFromSubject(subject string) string {
	// subject format: "vehicles.<vin>.<topic>"
	parts := make([]string, 0, 3)
	start := 0
	for i := 0; i <= len(subject); i++ {
		if i == len(subject) || subject[i] == '.' {
			parts = append(parts, subject[start:i])
			start = i + 1
			if len(parts) == 3 {
				break
			}
		}
	}
	if len(parts) >= 2 {
		return parts[1]
	}
	return ""
}

// Drain drains the underlying NATS connection for graceful shutdown.
func (nc *NATSClient) Drain() error {
	if err := nc.conn.Drain(); err != nil {
		return fmt.Errorf("drain NATS connection: %w", err)
	}
	return nil
}
