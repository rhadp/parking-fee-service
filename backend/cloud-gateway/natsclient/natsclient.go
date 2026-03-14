// Package natsclient provides NATS client utilities for the cloud-gateway service.
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

// CommandSubject returns the NATS subject for publishing commands to a VIN.
func CommandSubject(vin string) string {
	return "vehicles." + vin + ".commands"
}

// ResponseSubject returns the NATS wildcard subject for command responses.
func ResponseSubject() string {
	return "vehicles.*.command_responses"
}

// TelemetrySubject returns the NATS wildcard subject for telemetry.
func TelemetrySubject() string {
	return "vehicles.*.telemetry"
}

// Connect establishes a connection to the NATS server with exponential backoff.
// It retries up to maxRetries times (delays: 1s, 2s, 4s, …) before returning an error.
func Connect(url string, maxRetries int) (*nats.Conn, error) {
	var lastErr error
	delay := time.Second
	for attempt := 0; attempt <= maxRetries; attempt++ {
		if attempt > 0 {
			slog.Info("retrying NATS connection", "attempt", attempt, "delay", delay)
			time.Sleep(delay)
			delay *= 2
		}
		nc, err := nats.Connect(url)
		if err == nil {
			return nc, nil
		}
		lastErr = err
		slog.Error("failed to connect to NATS", "url", url, "attempt", attempt, "error", err)
	}
	return nil, fmt.Errorf("failed to connect to NATS after %d attempts: %w", maxRetries, lastErr)
}

// PublishCommand publishes a command to the NATS subject for the given VIN.
// The bearer token is included as a NATS message header (Authorization: Bearer <token>).
func PublishCommand(nc *nats.Conn, vin string, cmd model.Command, bearerToken string) error {
	data, err := json.Marshal(cmd)
	if err != nil {
		return fmt.Errorf("marshal command: %w", err)
	}

	msg := &nats.Msg{
		Subject: CommandSubject(vin),
		Data:    data,
		Header:  nats.Header{},
	}
	msg.Header.Set("Authorization", "Bearer "+bearerToken)

	if err := nc.PublishMsg(msg); err != nil {
		return fmt.Errorf("publish command: %w", err)
	}
	return nil
}

// SubscribeResponses subscribes to the command responses wildcard subject and
// updates the store when responses arrive. Invalid JSON and unknown command IDs
// are logged and discarded.
func SubscribeResponses(nc *nats.Conn, s *store.Store) (*nats.Subscription, error) {
	sub, err := nc.Subscribe(ResponseSubject(), func(msg *nats.Msg) {
		var resp model.CommandResponse
		if err := json.Unmarshal(msg.Data, &resp); err != nil {
			slog.Error("invalid JSON in command response", "error", err, "subject", msg.Subject)
			return
		}

		if _, found := s.Get(resp.CommandID); !found {
			slog.Warn("received response for unknown command_id", "command_id", resp.CommandID)
			return
		}

		s.UpdateFromResponse(resp)
		slog.Info("command response received", "command_id", resp.CommandID, "status", resp.Status)
	})
	if err != nil {
		return nil, fmt.Errorf("subscribe responses: %w", err)
	}
	return sub, nil
}

// SubscribeTelemetry subscribes to the telemetry wildcard subject and logs
// incoming telemetry data. Invalid JSON is logged and discarded.
func SubscribeTelemetry(nc *nats.Conn) (*nats.Subscription, error) {
	sub, err := nc.Subscribe(TelemetrySubject(), func(msg *nats.Msg) {
		// Extract VIN from subject: vehicles.{vin}.telemetry
		parts := strings.Split(msg.Subject, ".")
		vin := ""
		if len(parts) == 3 {
			vin = parts[1]
		}

		// Validate JSON
		var telemetry map[string]any
		if err := json.Unmarshal(msg.Data, &telemetry); err != nil {
			slog.Warn("invalid JSON in telemetry", "vin", vin, "subject", msg.Subject, "error", err)
			return
		}

		slog.Info("telemetry received", "vin", vin, "data", telemetry)
	})
	if err != nil {
		return nil, fmt.Errorf("subscribe telemetry: %w", err)
	}
	return sub, nil
}
