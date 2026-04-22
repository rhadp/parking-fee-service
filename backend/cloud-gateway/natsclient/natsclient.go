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

// NATSClient manages the NATS connection and provides methods for
// publishing commands and subscribing to responses and telemetry.
type NATSClient struct {
	conn *nats.Conn
}

// Connect establishes a connection to the NATS server at the given URL,
// retrying with exponential backoff up to maxRetries attempts.
func Connect(url string, maxRetries int) (*NATSClient, error) {
	var lastErr error
	backoff := time.Second

	for attempt := 1; attempt <= maxRetries; attempt++ {
		conn, err := nats.Connect(url, nats.MaxReconnects(-1))
		if err == nil {
			return &NATSClient{conn: conn}, nil
		}
		lastErr = err

		if attempt < maxRetries {
			time.Sleep(backoff)
			backoff *= 2
			if backoff > 4*time.Second {
				backoff = 4 * time.Second
			}
		}
	}

	return nil, fmt.Errorf("failed to connect to NATS after %d attempts: %w", maxRetries, lastErr)
}

// PublishCommand publishes a command to the NATS subject for the given VIN,
// including the bearer token as a message header.
func (nc *NATSClient) PublishCommand(vin string, cmd model.Command, token string) error {
	data, err := json.Marshal(cmd)
	if err != nil {
		return fmt.Errorf("marshaling command: %w", err)
	}

	msg := &nats.Msg{
		Subject: "vehicles." + vin + ".commands",
		Data:    data,
		Header:  nats.Header{},
	}
	msg.Header.Set("Authorization", "Bearer "+token)

	return nc.conn.PublishMsg(msg)
}

// SubscribeResponses subscribes to command response messages from all vehicles
// and stores them in the provided Store.
func (nc *NATSClient) SubscribeResponses(s *store.Store) error {
	_, err := nc.conn.Subscribe("vehicles.*.command_responses", func(msg *nats.Msg) {
		var resp model.CommandResponse
		if err := json.Unmarshal(msg.Data, &resp); err != nil {
			slog.Error("failed to parse command response",
				"error", err, "subject", msg.Subject)
			return
		}
		s.StoreResponse(resp)
	})
	return err
}

// SubscribeTelemetry subscribes to telemetry messages from all vehicles
// and logs them.
func (nc *NATSClient) SubscribeTelemetry() error {
	_, err := nc.conn.Subscribe("vehicles.*.telemetry", func(msg *nats.Msg) {
		vin := extractVINFromSubject(msg.Subject)
		slog.Info("telemetry received", "vin", vin, "data", string(msg.Data))
	})
	return err
}

// Drain gracefully drains the NATS connection.
func (nc *NATSClient) Drain() error {
	return nc.conn.Drain()
}

// extractVINFromSubject extracts the VIN from a NATS subject like
// "vehicles.{vin}.telemetry".
func extractVINFromSubject(subject string) string {
	parts := strings.Split(subject, ".")
	if len(parts) >= 2 {
		return parts[1]
	}
	return ""
}
