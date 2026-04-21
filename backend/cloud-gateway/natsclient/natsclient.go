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

// NATSClient wraps a NATS connection for the cloud-gateway.
type NATSClient struct {
	conn *nats.Conn
}

// Connect establishes a connection to the NATS server with exponential backoff retry.
// It attempts up to maxRetries times, doubling the wait duration between each attempt
// starting from 1 second (1s, 2s, 4s, 8s, ...). No wait is added after the last attempt.
func Connect(url string, maxRetries int) (*NATSClient, error) {
	opts := []nats.Option{
		nats.Timeout(2 * time.Second),
		nats.MaxReconnects(-1), // unlimited runtime reconnects (06-REQ-5.E2)
		nats.ReconnectWait(2 * time.Second),
	}

	var lastErr error
	wait := time.Second
	for attempt := 1; attempt <= maxRetries; attempt++ {
		conn, err := nats.Connect(url, opts...)
		if err == nil {
			slog.Info("NATS connected", "url", url, "attempt", attempt)
			return &NATSClient{conn: conn}, nil
		}
		lastErr = err
		slog.Warn("NATS connection failed", "attempt", attempt, "maxRetries", maxRetries, "error", err)
		if attempt < maxRetries {
			time.Sleep(wait)
			wait *= 2
		}
	}
	return nil, fmt.Errorf("NATS connection failed after %d attempts: %w", maxRetries, lastErr)
}

// PublishCommand publishes a command to "vehicles.{vin}.commands" with the bearer token
// as an Authorization header (06-REQ-1.2).
func (nc *NATSClient) PublishCommand(vin string, cmd model.Command, token string) error {
	data, err := json.Marshal(cmd)
	if err != nil {
		return fmt.Errorf("marshal command: %w", err)
	}
	msg := &nats.Msg{
		Subject: fmt.Sprintf("vehicles.%s.commands", vin),
		Header:  nats.Header{},
		Data:    data,
	}
	msg.Header.Set("Authorization", "Bearer "+token)
	return nc.conn.PublishMsg(msg)
}

// SubscribeResponses subscribes to "vehicles.*.command_responses" and stores
// parsed CommandResponse values in the given store (06-REQ-5.1, 06-REQ-5.2).
func (nc *NATSClient) SubscribeResponses(s *store.Store) error {
	_, err := nc.conn.Subscribe("vehicles.*.command_responses", func(msg *nats.Msg) {
		var resp model.CommandResponse
		if err := json.Unmarshal(msg.Data, &resp); err != nil {
			slog.Error("invalid command response JSON", "subject", msg.Subject, "error", err)
			return
		}
		if resp.CommandID == "" {
			slog.Warn("command response missing command_id", "subject", msg.Subject)
			return
		}
		s.StoreResponse(resp)
		slog.Info("command response stored", "command_id", resp.CommandID, "status", resp.Status)
	})
	return err
}

// SubscribeTelemetry subscribes to "vehicles.*.telemetry" and logs the telemetry payload.
// No storage or aggregation is performed (06-REQ-5.3).
func (nc *NATSClient) SubscribeTelemetry() error {
	_, err := nc.conn.Subscribe("vehicles.*.telemetry", func(msg *nats.Msg) {
		// Extract VIN from subject: vehicles.{vin}.telemetry
		parts := strings.Split(msg.Subject, ".")
		vin := ""
		if len(parts) >= 2 {
			vin = parts[1]
		}
		slog.Info("telemetry received", "vin", vin, "payload", string(msg.Data))
	})
	return err
}

// Drain drains the NATS connection for graceful shutdown (06-REQ-8.2).
func (nc *NATSClient) Drain() error {
	return nc.conn.Drain()
}
