package natsclient

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"strings"
	"time"

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
	backoff := 1 * time.Second
	var lastErr error

	for attempt := 1; attempt <= maxRetries; attempt++ {
		conn, err := nats.Connect(url, nats.MaxReconnects(-1))
		if err == nil {
			return &NATSClient{conn: conn}, nil
		}
		lastErr = err
		slog.Warn("NATS connection failed, retrying",
			"attempt", attempt,
			"max_retries", maxRetries,
			"backoff", backoff,
			"error", err,
		)
		if attempt < maxRetries {
			time.Sleep(backoff)
			backoff *= 2
		}
	}

	return nil, fmt.Errorf("failed to connect to NATS after %d attempts: %w", maxRetries, lastErr)
}

// PublishCommand publishes a command to the NATS subject for the given VIN,
// including the bearer token as an Authorization header.
func (nc *NATSClient) PublishCommand(vin string, cmd model.Command, token string) error {
	data, err := json.Marshal(cmd)
	if err != nil {
		return fmt.Errorf("failed to marshal command: %w", err)
	}

	msg := &nats.Msg{
		Subject: fmt.Sprintf("vehicles.%s.commands", vin),
		Data:    data,
		Header:  nats.Header{},
	}
	msg.Header.Set("Authorization", "Bearer "+token)

	return nc.conn.PublishMsg(msg)
}

// SubscribeResponses subscribes to command responses and stores them.
func (nc *NATSClient) SubscribeResponses(s *store.Store) error {
	_, err := nc.conn.Subscribe("vehicles.*.command_responses", func(msg *nats.Msg) {
		var resp model.CommandResponse
		if err := json.Unmarshal(msg.Data, &resp); err != nil {
			slog.Error("failed to parse command response",
				"error", err,
				"subject", msg.Subject,
			)
			return
		}

		// Extract VIN from subject for logging.
		parts := strings.Split(msg.Subject, ".")
		vin := ""
		if len(parts) >= 2 {
			vin = parts[1]
		}

		slog.Info("received command response",
			"command_id", resp.CommandID,
			"status", resp.Status,
			"vin", vin,
		)

		s.StoreResponse(resp)
	})
	return err
}

// SubscribeTelemetry subscribes to telemetry messages and logs them.
func (nc *NATSClient) SubscribeTelemetry() error {
	_, err := nc.conn.Subscribe("vehicles.*.telemetry", func(msg *nats.Msg) {
		// Extract VIN from subject.
		parts := strings.Split(msg.Subject, ".")
		vin := ""
		if len(parts) >= 2 {
			vin = parts[1]
		}

		slog.Info("received telemetry",
			"vin", vin,
			"data", string(msg.Data),
		)
	})
	return err
}

// Drain gracefully drains the NATS connection.
func (nc *NATSClient) Drain() error {
	return nc.conn.Drain()
}
