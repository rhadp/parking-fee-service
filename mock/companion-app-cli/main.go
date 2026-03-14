package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"
)

// config holds runtime configuration for companion-app-cli.
type config struct {
	GatewayURL string // CLOUD_GATEWAY base URL
	Token      string // Bearer token
}

// loadConfig reads configuration from environment variables and flags.
// Flags take precedence over environment variables, which take precedence
// over defaults. Satisfies: 09-REQ-5.2
func loadConfig(args []string) config {
	cfg := config{
		GatewayURL: "http://localhost:8081",
	}
	// Read env vars.
	if v := os.Getenv("CLOUD_GATEWAY_URL"); v != "" {
		cfg.GatewayURL = v
	}
	if v := os.Getenv("CLOUD_GATEWAY_TOKEN"); v != "" {
		cfg.Token = v
	}
	// Parse flags from args.
	for _, arg := range args {
		switch {
		case strings.HasPrefix(arg, "--gateway-url="):
			cfg.GatewayURL = arg[len("--gateway-url="):]
		case strings.HasPrefix(arg, "--token="):
			cfg.Token = arg[len("--token="):]
		}
	}
	return cfg
}

// run is the testable entry-point for companion-app-cli.
// Returns the process exit code.
func run(args []string, stdout, stderr io.Writer) int {
	if len(args) == 0 || args[0] == "--help" || args[0] == "-h" {
		printUsage(stderr)
		if len(args) > 0 {
			return 0
		}
		return 1
	}

	subcommand := args[0]
	rest := args[1:]

	// Parse per-subcommand flags and shared config.
	var vin, commandID string
	for _, arg := range rest {
		switch {
		case strings.HasPrefix(arg, "--vin="):
			vin = arg[len("--vin="):]
		case strings.HasPrefix(arg, "--command-id="):
			commandID = arg[len("--command-id="):]
		}
	}

	cfg := loadConfig(rest)

	// Validate token for subcommands that need it.
	if subcommand != "--help" && subcommand != "-h" {
		if cfg.Token == "" {
			fmt.Fprintln(stderr, "error: bearer token required; set --token flag or CLOUD_GATEWAY_TOKEN env var")
			return 1
		}
	}

	client := &http.Client{}

	switch subcommand {
	case "lock", "unlock":
		if vin == "" {
			fmt.Fprintln(stderr, "error: --vin is required")
			return 1
		}
		return cmdSendCommand(client, cfg, vin, subcommand, stdout, stderr)

	case "status":
		if vin == "" {
			fmt.Fprintln(stderr, "error: --vin is required")
			return 1
		}
		if commandID == "" {
			fmt.Fprintln(stderr, "error: --command-id is required")
			return 1
		}
		return cmdGetStatus(client, cfg, vin, commandID, stdout, stderr)

	default:
		fmt.Fprintf(stderr, "unknown subcommand: %q\n", subcommand)
		printUsage(stderr)
		return 1
	}
}

// cmdSendCommand sends a lock or unlock command to CLOUD_GATEWAY.
// Satisfies: 09-REQ-3.1, 09-REQ-3.2
func cmdSendCommand(client *http.Client, cfg config, vin, cmdType string, stdout, stderr io.Writer) int {
	commandID := newUUID()
	payload := map[string]any{
		"command_id": commandID,
		"type":       cmdType,
		"doors":      []string{"driver"},
	}
	body, _ := json.Marshal(payload)

	url := fmt.Sprintf("%s/vehicles/%s/commands", cfg.GatewayURL, vin)
	req, err := http.NewRequest("POST", url, strings.NewReader(string(body)))
	if err != nil {
		fmt.Fprintf(stderr, "error: failed to create request: %v\n", err)
		return 1
	}
	req.Header.Set("Authorization", "Bearer "+cfg.Token)
	req.Header.Set("Content-Type", "application/json")

	resp, err := client.Do(req)
	if err != nil {
		fmt.Fprintf(stderr, "error: failed to connect to CLOUD_GATEWAY at %s: %v\n", cfg.GatewayURL, err)
		return 1
	}
	defer resp.Body.Close()

	respBody, _ := io.ReadAll(resp.Body)
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		fmt.Fprintf(stderr, "error: CLOUD_GATEWAY returned HTTP %d: %s\n", resp.StatusCode, string(respBody))
		return 1
	}

	fmt.Fprintln(stdout, string(respBody))
	return 0
}

// cmdGetStatus queries the status of a previously submitted command.
// Satisfies: 09-REQ-3.3
func cmdGetStatus(client *http.Client, cfg config, vin, commandID string, stdout, stderr io.Writer) int {
	url := fmt.Sprintf("%s/vehicles/%s/commands/%s", cfg.GatewayURL, vin, commandID)
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		fmt.Fprintf(stderr, "error: failed to create request: %v\n", err)
		return 1
	}
	req.Header.Set("Authorization", "Bearer "+cfg.Token)

	resp, err := client.Do(req)
	if err != nil {
		fmt.Fprintf(stderr, "error: failed to connect to CLOUD_GATEWAY at %s: %v\n", cfg.GatewayURL, err)
		return 1
	}
	defer resp.Body.Close()

	respBody, _ := io.ReadAll(resp.Body)
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		fmt.Fprintf(stderr, "error: CLOUD_GATEWAY returned HTTP %d: %s\n", resp.StatusCode, string(respBody))
		return 1
	}

	fmt.Fprintln(stdout, string(respBody))
	return 0
}

func printUsage(w io.Writer) {
	fmt.Fprintln(w, "Usage: companion-app-cli <subcommand> [options]")
	fmt.Fprintln(w, "")
	fmt.Fprintln(w, "Subcommands:")
	fmt.Fprintln(w, "  lock   --vin=<vin> [--token=<token>] [--gateway-url=<url>]")
	fmt.Fprintln(w, "  unlock --vin=<vin> [--token=<token>] [--gateway-url=<url>]")
	fmt.Fprintln(w, "  status --vin=<vin> --command-id=<id> [--token=<token>] [--gateway-url=<url>]")
	fmt.Fprintln(w, "")
	fmt.Fprintln(w, "Environment variables:")
	fmt.Fprintln(w, "  CLOUD_GATEWAY_URL    CLOUD_GATEWAY base URL (default http://localhost:8081)")
	fmt.Fprintln(w, "  CLOUD_GATEWAY_TOKEN  Bearer token for authentication")
}

// newUUID generates a simple UUID v4-like string. In the real implementation
// this will use github.com/google/uuid.
func newUUID() string {
	return "00000000-0000-0000-0000-000000000001"
}

func main() {
	os.Exit(run(os.Args[1:], os.Stdout, os.Stderr))
}
