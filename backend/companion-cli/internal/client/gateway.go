// Package client provides HTTP client for CLOUD_GATEWAY REST API.
package client

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

// CommandResponse represents the response from submitting a command.
type CommandResponse struct {
	CommandID string `json:"command_id"`
	Status    string `json:"status"`
	RequestID string `json:"request_id"`
}

// CommandStatusResponse represents the status of a command.
type CommandStatusResponse struct {
	CommandID    string  `json:"command_id"`
	CommandType  string  `json:"command_type"`
	Status       string  `json:"status"`
	CreatedAt    string  `json:"created_at"`
	CompletedAt  *string `json:"completed_at,omitempty"`
	ErrorCode    string  `json:"error_code,omitempty"`
	ErrorMessage string  `json:"error_message,omitempty"`
	RequestID    string  `json:"request_id"`
}

// SubmitCommandRequest is the request body for command submission.
type SubmitCommandRequest struct {
	CommandType string   `json:"command_type"`
	Doors       []string `json:"doors"`
	AuthToken   string   `json:"auth_token"`
}

// ErrorResponse represents an error response from the API.
type ErrorResponse struct {
	ErrorCode string `json:"error_code"`
	Message   string `json:"message"`
	RequestID string `json:"request_id"`
}

// GatewayClient handles communication with CLOUD_GATEWAY REST API.
type GatewayClient struct {
	baseURL    string
	vin        string
	httpClient *http.Client
	authToken  string
}

// NewGatewayClient creates a new GatewayClient.
func NewGatewayClient(baseURL, vin string, timeout time.Duration) *GatewayClient {
	return &GatewayClient{
		baseURL: baseURL,
		vin:     vin,
		httpClient: &http.Client{
			Timeout: timeout,
		},
		authToken: "demo-auth-token", // Default auth token for demo
	}
}

// SetAuthToken sets the auth token for commands.
func (c *GatewayClient) SetAuthToken(token string) {
	c.authToken = token
}

// SendLockCommand sends a lock command for the configured VIN.
func (c *GatewayClient) SendLockCommand(ctx context.Context) (*CommandResponse, error) {
	return c.sendCommand(ctx, "lock")
}

// SendUnlockCommand sends an unlock command for the configured VIN.
func (c *GatewayClient) SendUnlockCommand(ctx context.Context) (*CommandResponse, error) {
	return c.sendCommand(ctx, "unlock")
}

// sendCommand sends a lock or unlock command.
func (c *GatewayClient) sendCommand(ctx context.Context, commandType string) (*CommandResponse, error) {
	url := fmt.Sprintf("%s/api/v1/vehicles/%s/commands", c.baseURL, c.vin)

	reqBody := SubmitCommandRequest{
		CommandType: commandType,
		Doors:       []string{"all"},
		AuthToken:   c.authToken,
	}

	bodyBytes, err := json.Marshal(reqBody)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request: %w", err)
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(bodyBytes))
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("cannot connect to %s: %w", c.baseURL, err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read response: %w", err)
	}

	if resp.StatusCode >= 400 {
		var errResp ErrorResponse
		if json.Unmarshal(body, &errResp) == nil && errResp.Message != "" {
			return nil, fmt.Errorf("HTTP %d: %s - %s", resp.StatusCode, errResp.ErrorCode, errResp.Message)
		}
		return nil, fmt.Errorf("HTTP %d: %s", resp.StatusCode, string(body))
	}

	var cmdResp CommandResponse
	if err := json.Unmarshal(body, &cmdResp); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	return &cmdResp, nil
}

// GetCommandStatus retrieves the status of a command by ID.
func (c *GatewayClient) GetCommandStatus(ctx context.Context, commandID string) (*CommandStatusResponse, error) {
	url := fmt.Sprintf("%s/api/v1/vehicles/%s/commands/%s", c.baseURL, c.vin, commandID)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("cannot connect to %s: %w", c.baseURL, err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read response: %w", err)
	}

	if resp.StatusCode >= 400 {
		var errResp ErrorResponse
		if json.Unmarshal(body, &errResp) == nil && errResp.Message != "" {
			return nil, fmt.Errorf("HTTP %d: %s - %s", resp.StatusCode, errResp.ErrorCode, errResp.Message)
		}
		return nil, fmt.Errorf("HTTP %d: %s", resp.StatusCode, string(body))
	}

	var statusResp CommandStatusResponse
	if err := json.Unmarshal(body, &statusResp); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	return &statusResp, nil
}

// Ping tests connectivity to the CLOUD_GATEWAY.
func (c *GatewayClient) Ping(ctx context.Context) error {
	url := fmt.Sprintf("%s/health", c.baseURL)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}

	start := time.Now()
	resp, err := c.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("cannot connect to %s: %w", c.baseURL, err)
	}
	defer resp.Body.Close()
	latency := time.Since(start)

	if resp.StatusCode >= 400 {
		return fmt.Errorf("HTTP %d from %s", resp.StatusCode, url)
	}

	_ = latency // Can be used for reporting
	return nil
}

// GetBaseURL returns the base URL.
func (c *GatewayClient) GetBaseURL() string {
	return c.baseURL
}

// GetVIN returns the configured VIN.
func (c *GatewayClient) GetVIN() string {
	return c.vin
}
