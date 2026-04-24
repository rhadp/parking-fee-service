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

// NATSClient wraps a NATS connection for command publishing and response subscription.
type NATSClient struct {
	conn *nats.Conn
}

// Connect establishes a NATS connection with exponential backoff retry.
// It attempts up to maxRetries connections, sleeping with exponential backoff
// (1s, 2s, 4s, 8s, ...) between attempts. Returns an error if all attempts fail.
func Connect(url string, maxRetries int) (*NATSClient, error) {
	var lastErr error
	backoff := time.Second

	for attempt := range maxRetries {
		conn, err := nats.Connect(url,
			nats.MaxReconnects(-1),
			nats.ReconnectWait(2*time.Second),
		)
		if err == nil {
			return &NATSClient{conn: conn}, nil
		}
		lastErr = err

		slog.Warn("NATS connection attempt failed",
			"attempt", attempt+1,
			"max_retries", maxRetries,
			"error", err,
		)

		if attempt < maxRetries-1 {
			time.Sleep(backoff)
			backoff *= 2
		}
	}

	return nil, fmt.Errorf("failed to connect to NATS after %d attempts: %w", maxRetries, lastErr)
}

// PublishCommand publishes a command to the NATS subject vehicles.{vin}.commands
// with the bearer token included as an Authorization header on the NATS message.
func (nc *NATSClient) PublishCommand(vin string, cmd model.Command, token string) error {
	subject := "vehicles." + vin + ".commands"

	data, err := json.Marshal(cmd)
	if err != nil {
		return fmt.Errorf("marshaling command: %w", err)
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

	return nil
}

// SubscribeResponses subscribes to vehicles.*.command_responses and stores
// received responses in the provided store.
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
		s.StoreResponse(resp)
		slog.Info("stored command response",
			"command_id", resp.CommandID,
			"status", resp.Status,
		)
	})
	return err
}

// SubscribeTelemetry subscribes to vehicles.*.telemetry and logs received
// telemetry data. No storage or aggregation is performed.
func (nc *NATSClient) SubscribeTelemetry() error {
	_, err := nc.conn.Subscribe("vehicles.*.telemetry", func(msg *nats.Msg) {
		// Extract VIN from subject (vehicles.{vin}.telemetry).
		parts := strings.Split(msg.Subject, ".")
		vin := ""
		if len(parts) >= 2 {
			vin = parts[1]
		}
		slog.Info("received telemetry",
			"vin", vin,
			"subject", msg.Subject,
			"data", string(msg.Data),
		)
	})
	return err
}

// Drain gracefully drains the NATS connection, allowing pending messages
// to be processed before closing.
func (nc *NATSClient) Drain() error {
	if nc.conn != nil {
		return nc.conn.Drain()
	}
	return nil
}
