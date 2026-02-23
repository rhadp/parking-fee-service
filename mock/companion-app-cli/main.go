package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"

	// Import generated proto packages to satisfy TS-01-27 (01-REQ-5.5)
	_ "github.com/rhadp/parking-fee-service/gen/go/commonpb"

	"github.com/google/uuid"
	"github.com/spf13/cobra"
)

var (
	gatewayURL string
	vin        string
	token      string
)

func main() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintln(os.Stderr, "Error:", err)
		os.Exit(1)
	}
}

var rootCmd = &cobra.Command{
	Use:   "companion-app-cli",
	Short: "Mock COMPANION_APP CLI for integration testing",
	Long:  "A mock CLI application that simulates the COMPANION_APP by exposing the same REST interfaces. Used for integration testing without real Android builds.",
}

func init() {
	rootCmd.PersistentFlags().StringVar(&gatewayURL, "gateway-url", "http://localhost:8081", "CLOUD_GATEWAY URL")
	rootCmd.PersistentFlags().StringVar(&vin, "vin", "VIN12345", "Vehicle identification number")
	rootCmd.PersistentFlags().StringVar(&token, "token", "", "Bearer token for authentication")

	rootCmd.AddCommand(lockCmd)
	rootCmd.AddCommand(unlockCmd)
	rootCmd.AddCommand(statusCmd)
}

// validateToken checks that the --token flag is provided. If not, it prints an
// error to stderr and exits with a non-zero exit code.
// Returns true if valid, false if missing (and the command should abort).
func validateToken() error {
	if token == "" {
		return fmt.Errorf("token is required: use --token flag to provide a bearer token")
	}
	return nil
}

// sendCommand sends a lock or unlock command to the CLOUD_GATEWAY.
// 03-REQ-4.1, 03-REQ-4.2, 03-REQ-4.4, 03-REQ-4.5, 03-REQ-4.6, 03-REQ-4.7
func sendCommand(commandType string) error {
	if err := validateToken(); err != nil {
		return err
	}

	commandID := uuid.New().String()

	body := map[string]interface{}{
		"command_id": commandID,
		"type":       commandType,
		"doors":      []string{"driver"},
	}

	jsonBody, err := json.Marshal(body)
	if err != nil {
		return fmt.Errorf("failed to marshal request body: %w", err)
	}

	url := fmt.Sprintf("%s/vehicles/%s/commands", gatewayURL, vin)

	req, err := http.NewRequest(http.MethodPost, url, bytes.NewReader(jsonBody))
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return fmt.Errorf("connection error: %w", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed to read response: %w", err)
	}

	if resp.StatusCode >= 400 {
		return fmt.Errorf("server error (HTTP %d): %s", resp.StatusCode, string(respBody))
	}

	fmt.Fprintln(os.Stdout, string(respBody))
	return nil
}

// queryStatus sends a GET request to the CLOUD_GATEWAY status endpoint.
// 03-REQ-4.3, 03-REQ-4.4, 03-REQ-4.5, 03-REQ-4.6, 03-REQ-4.7
func queryStatus() error {
	if err := validateToken(); err != nil {
		return err
	}

	url := fmt.Sprintf("%s/vehicles/%s/status", gatewayURL, vin)

	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return fmt.Errorf("connection error: %w", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed to read response: %w", err)
	}

	if resp.StatusCode >= 400 {
		return fmt.Errorf("server error (HTTP %d): %s", resp.StatusCode, string(respBody))
	}

	fmt.Fprintln(os.Stdout, string(respBody))
	return nil
}

// lockCmd sends a lock command to the CLOUD_GATEWAY.
// 03-REQ-4.1
var lockCmd = &cobra.Command{
	Use:   "lock",
	Short: "Send lock command via CLOUD_GATEWAY",
	RunE: func(cmd *cobra.Command, args []string) error {
		return sendCommand("lock")
	},
	SilenceUsage:  true,
	SilenceErrors: true,
}

// unlockCmd sends an unlock command to the CLOUD_GATEWAY.
// 03-REQ-4.2
var unlockCmd = &cobra.Command{
	Use:   "unlock",
	Short: "Send unlock command via CLOUD_GATEWAY",
	RunE: func(cmd *cobra.Command, args []string) error {
		return sendCommand("unlock")
	},
	SilenceUsage:  true,
	SilenceErrors: true,
}

// statusCmd queries vehicle status from the CLOUD_GATEWAY.
// 03-REQ-4.3
var statusCmd = &cobra.Command{
	Use:   "status",
	Short: "Query vehicle status via CLOUD_GATEWAY",
	RunE: func(cmd *cobra.Command, args []string) error {
		return queryStatus()
	},
	SilenceUsage:  true,
	SilenceErrors: true,
}
