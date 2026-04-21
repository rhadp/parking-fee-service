// Package main implements companion-app-cli, a mock for the COMPANION_APP on a mobile device.
// It sends lock/unlock commands and queries command status via CLOUD_GATEWAY REST API.
package main

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"
)

// config holds resolved runtime configuration.
type config struct {
	gatewayAddr string
	token       string
}

// resolveConfig reads flag and environment variable values, applying priority:
// flag > env > default.
func resolveConfig(gatewayAddr, token string) (config, error) {
	addr := gatewayAddr
	if addr == "" {
		addr = os.Getenv("CLOUD_GATEWAY_ADDR")
	}
	if addr == "" {
		addr = "http://localhost:8081"
	}

	tok := token
	if tok == "" {
		tok = os.Getenv("CLOUD_GATEWAY_TOKEN")
	}
	if tok == "" {
		return config{}, fmt.Errorf("bearer token is required: provide --token flag or set CLOUD_GATEWAY_TOKEN environment variable")
	}

	return config{gatewayAddr: addr, token: tok}, nil
}

// doRequest executes an HTTP request with the bearer token and returns the response body.
// On non-2xx status, it writes the status and body to stderr and returns an error.
func doRequest(req *http.Request, token string) ([]byte, error) {
	req.Header.Set("Authorization", "Bearer "+token)
	req.Header.Set("Content-Type", "application/json")

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("HTTP request failed: %w", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("read response body: %w", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, fmt.Errorf("HTTP %d: %s", resp.StatusCode, strings.TrimSpace(string(body)))
	}

	return body, nil
}

// cmdLock sends a POST /vehicles/{vin}/commands with type=lock.
func cmdLock(cfg config, vin string) error {
	body := map[string]any{
		"type":  "lock",
		"doors": []string{"driver"},
	}
	return postCommand(cfg, vin, body)
}

// cmdUnlock sends a POST /vehicles/{vin}/commands with type=unlock.
func cmdUnlock(cfg config, vin string) error {
	body := map[string]any{
		"type":  "unlock",
		"doors": []string{"driver"},
	}
	return postCommand(cfg, vin, body)
}

// postCommand encodes body as JSON and POSTs to /vehicles/{vin}/commands.
func postCommand(cfg config, vin string, body map[string]any) error {
	encoded, err := json.Marshal(body)
	if err != nil {
		return fmt.Errorf("encode request body: %w", err)
	}

	url := cfg.gatewayAddr + "/vehicles/" + vin + "/commands"
	req, err := http.NewRequest(http.MethodPost, url, bytes.NewReader(encoded))
	if err != nil {
		return fmt.Errorf("create request: %w", err)
	}

	respBody, err := doRequest(req, cfg.token)
	if err != nil {
		return err
	}

	fmt.Println(string(respBody))
	return nil
}

// cmdStatus sends GET /vehicles/{vin}/commands/{commandID} and prints the response.
func cmdStatus(cfg config, vin, commandID string) error {
	url := cfg.gatewayAddr + "/vehicles/" + vin + "/commands/" + commandID
	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		return fmt.Errorf("create request: %w", err)
	}

	respBody, err := doRequest(req, cfg.token)
	if err != nil {
		return err
	}

	fmt.Println(string(respBody))
	return nil
}

func main() {
	if len(os.Args) < 2 {
		// No arguments → print version and exit 0 (01-REQ-4.2, 01-REQ-4.4).
		fmt.Println("companion-app-cli v0.1.0")
		return
	}

	subcommand := os.Args[1]

	switch subcommand {
	case "lock", "unlock":
		fs := flag.NewFlagSet(subcommand, flag.ContinueOnError)
		fs.SetOutput(os.Stderr)
		vin := fs.String("vin", "", "Vehicle Identification Number (required)")
		token := fs.String("token", "", "Bearer token for Authorization header")
		gatewayAddr := fs.String("gateway-addr", "", "CLOUD_GATEWAY address (default: http://localhost:8081)")

		if err := fs.Parse(os.Args[2:]); err != nil {
			os.Exit(1)
		}

		if *vin == "" {
			fmt.Fprintln(os.Stderr, "error: --vin is required")
			fs.Usage()
			os.Exit(1)
		}

		cfg, err := resolveConfig(*gatewayAddr, *token)
		if err != nil {
			fmt.Fprintln(os.Stderr, "error:", err)
			os.Exit(1)
		}

		var cmdErr error
		if subcommand == "lock" {
			cmdErr = cmdLock(cfg, *vin)
		} else {
			cmdErr = cmdUnlock(cfg, *vin)
		}
		if cmdErr != nil {
			fmt.Fprintln(os.Stderr, "error:", cmdErr)
			os.Exit(1)
		}

	case "status":
		fs := flag.NewFlagSet("status", flag.ContinueOnError)
		fs.SetOutput(os.Stderr)
		vin := fs.String("vin", "", "Vehicle Identification Number (required)")
		commandID := fs.String("command-id", "", "Command ID to query (required)")
		token := fs.String("token", "", "Bearer token for Authorization header")
		gatewayAddr := fs.String("gateway-addr", "", "CLOUD_GATEWAY address (default: http://localhost:8081)")

		if err := fs.Parse(os.Args[2:]); err != nil {
			os.Exit(1)
		}

		if *vin == "" {
			fmt.Fprintln(os.Stderr, "error: --vin is required")
			fs.Usage()
			os.Exit(1)
		}
		if *commandID == "" {
			fmt.Fprintln(os.Stderr, "error: --command-id is required")
			fs.Usage()
			os.Exit(1)
		}

		cfg, err := resolveConfig(*gatewayAddr, *token)
		if err != nil {
			fmt.Fprintln(os.Stderr, "error:", err)
			os.Exit(1)
		}

		if err := cmdStatus(cfg, *vin, *commandID); err != nil {
			fmt.Fprintln(os.Stderr, "error:", err)
			os.Exit(1)
		}

	default:
		fmt.Fprintf(os.Stderr, "unknown command: %s\n", subcommand)
		fmt.Fprintln(os.Stderr, "usage: companion-app-cli <command> [flags]")
		fmt.Fprintln(os.Stderr, "commands: lock, unlock, status")
		os.Exit(1)
	}
}
