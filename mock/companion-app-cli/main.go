// Package main implements the companion-app-cli mock application.
//
// This CLI simulates the COMPANION_APP Android application by invoking REST
// calls against the CloudGateway service. It exercises the same REST API
// endpoints that the real companion app will use.
package main

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"time"
)

// Global configuration populated from flags / environment.
var (
	gatewayAddr string
	vin         string
	token       string
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}

// run parses arguments and dispatches to the appropriate subcommand.
func run(args []string) error {
	remaining, err := parseGlobalFlags(args)
	if err != nil {
		return err
	}

	if len(remaining) == 0 {
		printUsage()
		return fmt.Errorf("no command specified")
	}

	cmd := remaining[0]

	// Handle help before requiring --vin.
	switch cmd {
	case "help", "--help", "-h":
		printUsage()
		return nil
	}

	// --vin is required for all operational commands.
	if vin == "" {
		return fmt.Errorf("--vin is required")
	}

	switch cmd {
	case "lock":
		return cmdLock()
	case "unlock":
		return cmdUnlock()
	case "status":
		return cmdStatus()
	default:
		printUsage()
		return fmt.Errorf("unknown command: %s", cmd)
	}
}

// parseGlobalFlags extracts global flags and returns remaining arguments.
func parseGlobalFlags(args []string) ([]string, error) {
	gatewayAddr = envOrDefault("GATEWAY_ADDR", "http://localhost:8081")
	vin = os.Getenv("VIN")
	token = envOrDefault("TOKEN", "demo-token")

	var remaining []string
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--gateway-addr":
			if i+1 >= len(args) {
				return nil, fmt.Errorf("--gateway-addr requires a value")
			}
			i++
			gatewayAddr = args[i]
		case "--vin":
			if i+1 >= len(args) {
				return nil, fmt.Errorf("--vin requires a value")
			}
			i++
			vin = args[i]
		case "--token":
			if i+1 >= len(args) {
				return nil, fmt.Errorf("--token requires a value")
			}
			i++
			token = args[i]
		default:
			remaining = append(remaining, args[i])
		}
	}
	return remaining, nil
}

func printUsage() {
	fmt.Fprintf(os.Stderr, `Usage: companion-app-cli [flags] <command>

Commands:
  lock              POST /api/v1/vehicles/{vin}/lock
  unlock            POST /api/v1/vehicles/{vin}/unlock
  status            GET  /api/v1/vehicles/{vin}/status

Global Flags:
  --gateway-addr    Address of CloudGateway (default: http://localhost:8081)
  --vin             Vehicle VIN (required)
  --token           Bearer token (default: demo-token)
`)
}

// doRequest sends an HTTP request and prints the response body.
func doRequest(method, url string) error {
	client := &http.Client{Timeout: 10 * time.Second}

	req, err := http.NewRequest(method, url, nil)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("Authorization", "Bearer "+token)
	req.Header.Set("Accept", "application/json")

	resp, err := client.Do(req)
	if err != nil {
		return fmt.Errorf("request to %s failed: %w", url, err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed to read response body: %w", err)
	}

	fmt.Printf("HTTP %d %s\n", resp.StatusCode, resp.Status)
	if len(body) > 0 {
		fmt.Println(string(body))
	}
	return nil
}

func cmdLock() error {
	url := fmt.Sprintf("%s/api/v1/vehicles/%s/lock", gatewayAddr, vin)
	return doRequest(http.MethodPost, url)
}

func cmdUnlock() error {
	url := fmt.Sprintf("%s/api/v1/vehicles/%s/unlock", gatewayAddr, vin)
	return doRequest(http.MethodPost, url)
}

func cmdStatus() error {
	url := fmt.Sprintf("%s/api/v1/vehicles/%s/status", gatewayAddr, vin)
	return doRequest(http.MethodGet, url)
}

// envOrDefault returns the value of the given environment variable, or the
// default value if the variable is not set.
func envOrDefault(key, defaultVal string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return defaultVal
}
