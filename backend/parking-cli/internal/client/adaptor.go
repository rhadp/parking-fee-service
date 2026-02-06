package client

import (
	"context"
	"fmt"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"

	"github.com/sdv-parking-demo/backend/gen/services/parking"
)

// SessionInfo represents parking session information.
type SessionInfo struct {
	SessionID       string
	HasActiveSession bool
	State           string
	CurrentCost     float64
	StartTimeUnix   int64
	DurationSeconds int64
	ZoneID          string
	ErrorMessage    string
}

// StartSessionResult represents the result of starting a session.
type StartSessionResult struct {
	SessionID    string
	Success      bool
	ErrorMessage string
	State        string
}

// StopSessionResult represents the result of stopping a session.
type StopSessionResult struct {
	Success         bool
	ErrorMessage    string
	SessionID       string
	State           string
	FinalCost       float64
	DurationSeconds int64
}

// ParkingAdaptorClient handles communication with PARKING_OPERATOR_ADAPTOR gRPC service.
type ParkingAdaptorClient struct {
	conn    *grpc.ClientConn
	client  parking.ParkingAdaptorClient
	address string
}

// NewParkingAdaptorClient creates a new ParkingAdaptorClient.
func NewParkingAdaptorClient(address string, timeout time.Duration) (*ParkingAdaptorClient, error) {
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	conn, err := grpc.DialContext(ctx, address,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to PARKING_ADAPTOR at %s: %w", address, err)
	}

	return &ParkingAdaptorClient{
		conn:    conn,
		client:  parking.NewParkingAdaptorClient(conn),
		address: address,
	}, nil
}

// StartSession starts a new parking session.
func (c *ParkingAdaptorClient) StartSession(ctx context.Context, zoneID string) (*StartSessionResult, error) {
	resp, err := c.client.StartSession(ctx, &parking.StartSessionRequest{
		ZoneId: zoneID,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to start session: %w", err)
	}

	return &StartSessionResult{
		SessionID:    resp.SessionId,
		Success:      resp.Success,
		ErrorMessage: resp.ErrorMessage,
		State:        resp.State.String(),
	}, nil
}

// StopSession stops the current parking session.
func (c *ParkingAdaptorClient) StopSession(ctx context.Context) (*StopSessionResult, error) {
	resp, err := c.client.StopSession(ctx, &parking.StopSessionRequest{})
	if err != nil {
		return nil, fmt.Errorf("failed to stop session: %w", err)
	}

	return &StopSessionResult{
		Success:         resp.Success,
		ErrorMessage:    resp.ErrorMessage,
		SessionID:       resp.SessionId,
		State:           resp.State.String(),
		FinalCost:       resp.FinalCost,
		DurationSeconds: resp.DurationSeconds,
	}, nil
}

// GetSessionStatus retrieves current session status.
func (c *ParkingAdaptorClient) GetSessionStatus(ctx context.Context) (*SessionInfo, error) {
	resp, err := c.client.GetSessionStatus(ctx, &parking.GetSessionStatusRequest{})
	if err != nil {
		return nil, fmt.Errorf("failed to get session status: %w", err)
	}

	return &SessionInfo{
		SessionID:        resp.SessionId,
		HasActiveSession: resp.HasActiveSession,
		State:            resp.State.String(),
		CurrentCost:      resp.CurrentCost,
		StartTimeUnix:    resp.StartTimeUnix,
		DurationSeconds:  resp.DurationSeconds,
		ZoneID:           resp.ZoneId,
		ErrorMessage:     resp.ErrorMessage,
	}, nil
}

// Ping tests connectivity to the PARKING_ADAPTOR.
func (c *ParkingAdaptorClient) Ping(ctx context.Context) error {
	_, err := c.client.GetSessionStatus(ctx, &parking.GetSessionStatusRequest{})
	// Any response (including no session) indicates connectivity
	return err
}

// Close closes the gRPC connection.
func (c *ParkingAdaptorClient) Close() error {
	if c.conn != nil {
		return c.conn.Close()
	}
	return nil
}

// GetAddress returns the service address.
func (c *ParkingAdaptorClient) GetAddress() string {
	return c.address
}
