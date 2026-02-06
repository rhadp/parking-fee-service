// Package service provides business logic for the cloud-gateway service.
package service

import (
	"context"
	"encoding/json"
	"log/slog"
	"sync"
	"time"

	"github.com/google/uuid"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/audit"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/mqtt"
	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/store"
)

// CommandService handles command operations.
type CommandService struct {
	store           *store.CommandStore
	mqttClient      mqtt.Client
	auditLogger     audit.AuditLogger
	logger          *slog.Logger
	commandTimeout  time.Duration
	commandsTopic   string
	configuredVIN   string
	stopTimeoutChan chan struct{}
	wg              sync.WaitGroup
}

// NewCommandService creates a new CommandService.
func NewCommandService(
	store *store.CommandStore,
	mqttClient mqtt.Client,
	auditLogger audit.AuditLogger,
	logger *slog.Logger,
	commandTimeout time.Duration,
	configuredVIN string,
) *CommandService {
	return &CommandService{
		store:           store,
		mqttClient:      mqttClient,
		auditLogger:     auditLogger,
		logger:          logger,
		commandTimeout:  commandTimeout,
		commandsTopic:   "vehicles/" + configuredVIN + "/commands",
		configuredVIN:   configuredVIN,
		stopTimeoutChan: make(chan struct{}),
	}
}

// SubmitCommand creates a new command, stores it, and publishes to MQTT.
func (s *CommandService) SubmitCommand(ctx context.Context, req *model.SubmitCommandRequest, sourceIP string) (*model.Command, error) {
	// Generate unique command ID
	commandID := uuid.New().String()

	// Create command
	cmd := &model.Command{
		CommandID:   commandID,
		CommandType: req.CommandType,
		Doors:       req.Doors,
		AuthToken:   req.AuthToken,
		Status:      model.CommandStatusPending,
		CreatedAt:   time.Now(),
	}

	// Store command
	s.store.Save(cmd)

	// Create MQTT message
	mqttMsg := &model.MQTTCommandMessage{
		CommandID: commandID,
		Type:      req.CommandType,
		Doors:     req.Doors,
		AuthToken: req.AuthToken,
		Timestamp: time.Now().UTC().Format(time.RFC3339),
	}

	// Serialize and publish to MQTT
	payload, err := json.Marshal(mqttMsg)
	if err != nil {
		s.logger.Error("failed to marshal command message",
			slog.String("command_id", commandID),
			slog.String("error", err.Error()),
		)
		return nil, err
	}

	if err := s.mqttClient.Publish(s.commandsTopic, payload); err != nil {
		s.logger.Error("failed to publish command to MQTT",
			slog.String("command_id", commandID),
			slog.String("error", err.Error()),
		)
		// Update command status to failed
		cmd.Status = model.CommandStatusFailed
		cmd.ErrorCode = model.ErrInternalError
		cmd.ErrorMessage = "Failed to publish command"
		now := time.Now()
		cmd.CompletedAt = &now
		s.store.Update(cmd)
		return cmd, nil
	}

	// Log audit event
	if s.auditLogger != nil {
		event := &model.CommandSubmissionEvent{
			AuditEventBase: model.NewAuditEventBase(""),
			VIN:            s.configuredVIN,
			CommandType:    req.CommandType,
			Doors:          req.Doors,
			SourceIP:       sourceIP,
			CommandID:      commandID,
		}
		s.auditLogger.LogCommandSubmission(ctx, event)
	}

	s.logger.Info("command submitted",
		slog.String("command_id", commandID),
		slog.String("command_type", req.CommandType),
	)

	return cmd, nil
}

// GetCommandStatus retrieves the status of a command.
func (s *CommandService) GetCommandStatus(commandID string) *model.Command {
	return s.store.Get(commandID)
}

// HandleCommandResponse processes a command response from the vehicle.
func (s *CommandService) HandleCommandResponse(ctx context.Context, response *model.MQTTCommandResponse) {
	cmd := s.store.Get(response.CommandID)
	if cmd == nil {
		s.logger.Warn("received response for unknown command",
			slog.String("command_id", response.CommandID),
		)
		return
	}

	previousStatus := cmd.Status

	// Update command with response
	cmd.Status = response.Status
	cmd.ErrorCode = response.ErrorCode
	cmd.ErrorMessage = response.ErrorMessage
	now := time.Now()
	cmd.CompletedAt = &now

	s.store.Update(cmd)

	// Log audit event
	if s.auditLogger != nil {
		event := &model.CommandStatusChangeEvent{
			AuditEventBase: model.NewAuditEventBase(""),
			CommandID:      response.CommandID,
			PreviousStatus: previousStatus,
			NewStatus:      response.Status,
		}
		s.auditLogger.LogCommandStatusChange(ctx, event)
	}

	s.logger.Info("command status updated",
		slog.String("command_id", response.CommandID),
		slog.String("previous_status", previousStatus),
		slog.String("new_status", response.Status),
	)
}

// StartTimeoutChecker starts a background goroutine that checks for timed out commands.
func (s *CommandService) StartTimeoutChecker(interval time.Duration) {
	s.wg.Add(1)
	go func() {
		defer s.wg.Done()
		ticker := time.NewTicker(interval)
		defer ticker.Stop()

		for {
			select {
			case <-ticker.C:
				s.checkTimeouts()
			case <-s.stopTimeoutChan:
				return
			}
		}
	}()
}

// StopTimeoutChecker stops the timeout checker goroutine.
func (s *CommandService) StopTimeoutChecker() {
	close(s.stopTimeoutChan)
	s.wg.Wait()
}

// checkTimeouts checks for commands that have exceeded the timeout.
func (s *CommandService) checkTimeouts() {
	pending := s.store.GetPendingCommands()
	now := time.Now()

	for _, cmd := range pending {
		if now.Sub(cmd.CreatedAt) > s.commandTimeout {
			s.markTimeout(cmd)
		}
	}
}

// markTimeout marks a command as timed out.
func (s *CommandService) markTimeout(cmd *model.Command) {
	previousStatus := cmd.Status
	cmd.Status = model.CommandStatusTimeout
	cmd.ErrorCode = model.ErrTimeout
	cmd.ErrorMessage = "Command timed out waiting for response"
	now := time.Now()
	cmd.CompletedAt = &now

	s.store.Update(cmd)

	// Log audit event
	if s.auditLogger != nil {
		event := &model.CommandStatusChangeEvent{
			AuditEventBase: model.NewAuditEventBase(""),
			CommandID:      cmd.CommandID,
			PreviousStatus: previousStatus,
			NewStatus:      model.CommandStatusTimeout,
		}
		s.auditLogger.LogCommandStatusChange(context.Background(), event)
	}

	s.logger.Info("command timed out",
		slog.String("command_id", cmd.CommandID),
		slog.Duration("timeout", s.commandTimeout),
	)
}

// HandleCommandResponseFromMQTT implements the mqtt.CommandResponseHandler interface.
func (s *CommandService) HandleCommandResponseFromMQTT(response *model.MQTTCommandResponse) {
	s.HandleCommandResponse(context.Background(), response)
}
