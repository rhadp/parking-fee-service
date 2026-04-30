// Package natsclient manages NATS connections, publishing commands,
// and subscribing to responses and telemetry.
package natsclient

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"strings"
	"time"

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
// retrying with exponential backoff (1s, 2s, 4s, ...) up to maxRetries
// attempts. Returns an error if all attempts fail.
func Connect(url string, maxRetries int) (*NATSClient, error) {
	var lastErr error
	backoff := 1 * time.Second

	for attempt := 1; attempt <= maxRetries; attempt++ {
		conn, err := nats.Connect(url,
			nats.MaxReconnects(-1),
			nats.ReconnectWait(2*time.Second),
			nats.Timeout(2*time.Second),
		)
		if err == nil {
			slog.Info("connected to NATS", "url", url, "attempt", attempt)
			return &NATSClient{conn: conn}, nil
		}
		lastErr = err
		slog.Warn("NATS connection attempt failed",
			"url", url,
			"attempt", attempt,
			"maxRetries", maxRetries,
			"error", err,
		)
		if attempt < maxRetries {
			time.Sleep(backoff)
			backoff *= 2
		}
	}

	return nil, fmt.Errorf("failed to connect to NATS after %d attempts: %w", maxRetries, lastErr)
}

// PublishCommand publishes a command to the NATS subject vehicles.{vin}.commands
// with the bearer token in the Authorization header.
func (nc *NATSClient) PublishCommand(vin string, cmd model.Command, token string) error {
	subject := fmt.Sprintf("vehicles.%s.commands", vin)

	data, err := json.Marshal(cmd)
	if err != nil {
		return fmt.Errorf("marshalling command: %w", err)
	}

	msg := &nats.Msg{
		Subject: subject,
		Data:    data,
		Header:  nats.Header{},
	}
	msg.Header.Set("Authorization", "Bearer "+token)

	if err := nc.conn.PublishMsg(msg); err != nil {
		return fmt.Errorf("publishing command to NATS: %w", err)
	}

	slog.Info("published command to NATS",
		"subject", subject,
		"command_id", cmd.CommandID,
		"type", cmd.Type,
	)

	return nil
}

// SubscribeResponses subscribes to vehicles.*.command_responses and stores
// incoming responses in the provided store.
func (nc *NATSClient) SubscribeResponses(s *store.Store) error {
	_, err := nc.conn.Subscribe("vehicles.*.command_responses", func(msg *nats.Msg) {
		var resp model.CommandResponse
		if err := json.Unmarshal(msg.Data, &resp); err != nil {
			slog.Error("failed to parse command response",
				"subject", msg.Subject,
				"error", err,
			)
			return
		}

		// Extract VIN from subject for logging
		vin := extractVINFromSubject(msg.Subject)
		slog.Info("received command response",
			"vin", vin,
			"command_id", resp.CommandID,
			"status", resp.Status,
		)

		s.StoreResponse(resp)
	})
	if err != nil {
		return fmt.Errorf("subscribing to command responses: %w", err)
	}

	slog.Info("subscribed to command responses", "subject", "vehicles.*.command_responses")
	return nil
}

// SubscribeTelemetry subscribes to vehicles.*.telemetry and logs incoming data.
func (nc *NATSClient) SubscribeTelemetry() error {
	_, err := nc.conn.Subscribe("vehicles.*.telemetry", func(msg *nats.Msg) {
		vin := extractVINFromSubject(msg.Subject)
		slog.Info("received telemetry",
			"vin", vin,
			"data", string(msg.Data),
		)
	})
	if err != nil {
		return fmt.Errorf("subscribing to telemetry: %w", err)
	}

	slog.Info("subscribed to telemetry", "subject", "vehicles.*.telemetry")
	return nil
}

// Drain drains the NATS connection for graceful shutdown.
func (nc *NATSClient) Drain() error {
	if nc.conn != nil {
		return nc.conn.Drain()
	}
	return nil
}

// extractVINFromSubject extracts the VIN from a NATS subject of the form
// vehicles.{vin}.command_responses or vehicles.{vin}.telemetry.
func extractVINFromSubject(subject string) string {
	parts := strings.Split(subject, ".")
	if len(parts) >= 2 {
		return parts[1]
	}
	return ""
}
