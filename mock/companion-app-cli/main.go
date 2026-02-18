// Package main implements the companion-app-cli mock application.
//
// This CLI simulates the COMPANION_APP mobile application by invoking REST
// calls against the CLOUD_GATEWAY service. It supports vehicle pairing,
// lock/unlock commands, and status queries.
//
// Usage:
//
//	companion-app-cli [flags] <command>
//
// Commands:
//
//	pair     POST /api/v1/pair {vin, pin} - prints the returned token
//	lock     POST /api/v1/vehicles/{vin}/lock - sends a lock command
//	unlock   POST /api/v1/vehicles/{vin}/unlock - sends an unlock command
//	status   GET  /api/v1/vehicles/{vin}/status - prints vehicle state
//
// Global Flags:
//
//	--gateway-addr  CLOUD_GATEWAY address (default: http://localhost:8081)
//	--vin           Vehicle VIN (required for all commands)
//	--token         Bearer token (required for lock/unlock/status)
//	--pin           Pairing PIN (required for pair)
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"time"
)

// config holds CLI configuration populated from flags / environment.
type config struct {
	gatewayAddr string
	vin         string
	token       string
	pin         string
}

func main() {
	if err := run(os.Args[1:], os.Stdout, os.Stderr); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}

// run parses arguments and dispatches to the appropriate subcommand.
// stdout and stderr are used for output to allow testing.
func run(args []string, stdout, stderr io.Writer) error {
	cfg, remaining, err := parseGlobalFlags(args)
	if err != nil {
		return err
	}

	if len(remaining) == 0 {
		printUsage(stderr)
		return fmt.Errorf("no command specified")
	}

	cmd := remaining[0]

	// Handle help before requiring --vin.
	switch cmd {
	case "help", "--help", "-h":
		printUsage(stderr)
		return nil
	}

	// --vin is required for all operational commands.
	if cfg.vin == "" {
		return fmt.Errorf("--vin is required")
	}

	switch cmd {
	case "pair":
		return cmdPair(cfg, stdout)
	case "lock":
		return cmdLock(cfg, stdout)
	case "unlock":
		return cmdUnlock(cfg, stdout)
	case "status":
		return cmdStatus(cfg, stdout)
	default:
		printUsage(stderr)
		return fmt.Errorf("unknown command: %s", cmd)
	}
}

// parseGlobalFlags extracts global flags and returns the config plus remaining
// positional arguments.
func parseGlobalFlags(args []string) (*config, []string, error) {
	cfg := &config{
		gatewayAddr: envOrDefault("GATEWAY_ADDR", "http://localhost:8081"),
		vin:         os.Getenv("VIN"),
		token:       os.Getenv("TOKEN"),
		pin:         os.Getenv("PIN"),
	}

	var remaining []string
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--gateway-addr":
			if i+1 >= len(args) {
				return nil, nil, fmt.Errorf("--gateway-addr requires a value")
			}
			i++
			cfg.gatewayAddr = args[i]
		case "--vin":
			if i+1 >= len(args) {
				return nil, nil, fmt.Errorf("--vin requires a value")
			}
			i++
			cfg.vin = args[i]
		case "--token":
			if i+1 >= len(args) {
				return nil, nil, fmt.Errorf("--token requires a value")
			}
			i++
			cfg.token = args[i]
		case "--pin":
			if i+1 >= len(args) {
				return nil, nil, fmt.Errorf("--pin requires a value")
			}
			i++
			cfg.pin = args[i]
		default:
			remaining = append(remaining, args[i])
		}
	}
	return cfg, remaining, nil
}

func printUsage(w io.Writer) {
	fmt.Fprintf(w, `Usage: companion-app-cli [flags] <command>

Commands:
  pair              POST /api/v1/pair {vin, pin} - pair with vehicle
  lock              POST /api/v1/vehicles/{vin}/lock
  unlock            POST /api/v1/vehicles/{vin}/unlock
  status            GET  /api/v1/vehicles/{vin}/status

Global Flags:
  --gateway-addr    Address of CLOUD_GATEWAY (default: http://localhost:8081)
  --vin             Vehicle VIN (required for all commands)
  --token           Bearer token (required for lock/unlock/status)
  --pin             Pairing PIN (required for pair)
`)
}

// pairRequest is the JSON body for POST /api/v1/pair.
type pairRequest struct {
	VIN string `json:"vin"`
	PIN string `json:"pin"`
}

// pairResponse is the JSON response from POST /api/v1/pair.
type pairResponse struct {
	Token string `json:"token"`
	VIN   string `json:"vin"`
}

// cmdPair sends a pairing request to the CLOUD_GATEWAY.
func cmdPair(cfg *config, stdout io.Writer) error {
	if cfg.pin == "" {
		return fmt.Errorf("--pin is required for pair command")
	}

	body, err := json.Marshal(pairRequest{
		VIN: cfg.vin,
		PIN: cfg.pin,
	})
	if err != nil {
		return fmt.Errorf("failed to marshal pair request: %w", err)
	}

	url := fmt.Sprintf("%s/api/v1/pair", cfg.gatewayAddr)
	resp, respBody, err := doHTTPRequest(http.MethodPost, url, "", body)
	if err != nil {
		return err
	}

	if resp.StatusCode != http.StatusOK {
		return handleErrorResponse(resp.StatusCode, respBody)
	}

	var pr pairResponse
	if err := json.Unmarshal(respBody, &pr); err != nil {
		return fmt.Errorf("failed to parse pair response: %w", err)
	}

	fmt.Fprintf(stdout, "Paired successfully with vehicle %s\n", pr.VIN)
	fmt.Fprintf(stdout, "Token: %s\n", pr.Token)
	return nil
}

// cmdLock sends a lock command to the CLOUD_GATEWAY.
func cmdLock(cfg *config, stdout io.Writer) error {
	if cfg.token == "" {
		return fmt.Errorf("--token is required for lock command")
	}

	url := fmt.Sprintf("%s/api/v1/vehicles/%s/lock", cfg.gatewayAddr, cfg.vin)
	resp, respBody, err := doHTTPRequest(http.MethodPost, url, cfg.token, nil)
	if err != nil {
		return err
	}

	if resp.StatusCode != http.StatusAccepted {
		return handleErrorResponse(resp.StatusCode, respBody)
	}

	fmt.Fprintf(stdout, "Lock command accepted\n")
	fmt.Fprintf(stdout, "%s\n", string(respBody))
	return nil
}

// cmdUnlock sends an unlock command to the CLOUD_GATEWAY.
func cmdUnlock(cfg *config, stdout io.Writer) error {
	if cfg.token == "" {
		return fmt.Errorf("--token is required for unlock command")
	}

	url := fmt.Sprintf("%s/api/v1/vehicles/%s/unlock", cfg.gatewayAddr, cfg.vin)
	resp, respBody, err := doHTTPRequest(http.MethodPost, url, cfg.token, nil)
	if err != nil {
		return err
	}

	if resp.StatusCode != http.StatusAccepted {
		return handleErrorResponse(resp.StatusCode, respBody)
	}

	fmt.Fprintf(stdout, "Unlock command accepted\n")
	fmt.Fprintf(stdout, "%s\n", string(respBody))
	return nil
}

// cmdStatus queries the vehicle status from the CLOUD_GATEWAY.
func cmdStatus(cfg *config, stdout io.Writer) error {
	if cfg.token == "" {
		return fmt.Errorf("--token is required for status command")
	}

	url := fmt.Sprintf("%s/api/v1/vehicles/%s/status", cfg.gatewayAddr, cfg.vin)
	resp, respBody, err := doHTTPRequest(http.MethodGet, url, cfg.token, nil)
	if err != nil {
		return err
	}

	if resp.StatusCode != http.StatusOK {
		return handleErrorResponse(resp.StatusCode, respBody)
	}

	// Pretty-print the JSON response.
	var pretty bytes.Buffer
	if err := json.Indent(&pretty, respBody, "", "  "); err != nil {
		// Fall back to raw output.
		fmt.Fprintf(stdout, "%s\n", string(respBody))
		return nil
	}

	fmt.Fprintf(stdout, "Vehicle Status:\n%s\n", pretty.String())
	return nil
}

// doHTTPRequest sends an HTTP request and returns the response + body.
// If token is non-empty, it is sent as a Bearer token in the Authorization header.
// If body is non-nil, it is sent as the request body with Content-Type application/json.
func doHTTPRequest(method, url, token string, body []byte) (*http.Response, []byte, error) {
	client := &http.Client{Timeout: 10 * time.Second}

	var bodyReader io.Reader
	if body != nil {
		bodyReader = bytes.NewReader(body)
	}

	req, err := http.NewRequest(method, url, bodyReader)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to create request: %w", err)
	}

	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	req.Header.Set("Accept", "application/json")
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}

	resp, err := client.Do(req)
	if err != nil {
		return nil, nil, fmt.Errorf("request to %s failed (is the gateway running?): %w", url, err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to read response body: %w", err)
	}

	return resp, respBody, nil
}

// errorResponse represents an error returned by the CLOUD_GATEWAY REST API.
type errorResponse struct {
	Error string `json:"error"`
	Code  string `json:"code"`
}

// handleErrorResponse interprets an HTTP error response and returns a
// descriptive error.
func handleErrorResponse(statusCode int, body []byte) error {
	var errResp errorResponse
	if err := json.Unmarshal(body, &errResp); err == nil && errResp.Error != "" {
		return fmt.Errorf("HTTP %d: %s", statusCode, errResp.Error)
	}
	// Fall back to generic error if the response body isn't a valid error JSON.
	return fmt.Errorf("HTTP %d: %s", statusCode, http.StatusText(statusCode))
}

// envOrDefault returns the value of the given environment variable, or the
// default value if the variable is not set.
func envOrDefault(key, defaultVal string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return defaultVal
}
