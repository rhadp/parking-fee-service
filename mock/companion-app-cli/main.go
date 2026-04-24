package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}

// run parses the subcommand and flags, then dispatches to the appropriate handler.
func run(args []string) error {
	if len(args) == 0 {
		return fmt.Errorf("usage: companion-app-cli <lock|unlock|status> [flags]")
	}

	subcmd := args[0]
	flags := parseFlags(args[1:])

	// Resolve token — required for all operations.
	token := flagOrEnv(flags, "token", "CLOUD_GATEWAY_TOKEN")
	if token == "" {
		return fmt.Errorf("missing required token: provide --token flag or set CLOUD_GATEWAY_TOKEN environment variable")
	}

	// Resolve gateway address.
	addr := flagOrEnvDefault(flags, "gateway-addr", "CLOUD_GATEWAY_ADDR", "http://localhost:8081")

	switch subcmd {
	case "lock":
		return doCommand(addr, token, flags, "lock")
	case "unlock":
		return doCommand(addr, token, flags, "unlock")
	case "status":
		return doStatus(addr, token, flags)
	default:
		return fmt.Errorf("unknown subcommand: %s\nusage: companion-app-cli <lock|unlock|status> [flags]", subcmd)
	}
}

// doCommand sends a lock or unlock POST request to CLOUD_GATEWAY.
func doCommand(addr, token string, flags map[string]string, cmdType string) error {
	vin := flags["vin"]
	if vin == "" {
		return fmt.Errorf("missing required flag: --vin")
	}

	body := map[string]any{
		"type":  cmdType,
		"doors": []string{"driver"},
	}
	bodyBytes, _ := json.Marshal(body)

	url := fmt.Sprintf("%s/vehicles/%s/commands", addr, vin)
	return doHTTPRequest("POST", url, token, bodyBytes)
}

// doStatus sends a GET request for command status to CLOUD_GATEWAY.
func doStatus(addr, token string, flags map[string]string) error {
	vin := flags["vin"]
	if vin == "" {
		return fmt.Errorf("missing required flag: --vin")
	}
	commandID := flags["command-id"]
	if commandID == "" {
		return fmt.Errorf("missing required flag: --command-id")
	}

	url := fmt.Sprintf("%s/vehicles/%s/commands/%s", addr, vin, commandID)
	return doHTTPRequest("GET", url, token, nil)
}

// doHTTPRequest performs an HTTP request with bearer token auth and prints the
// JSON response to stdout.
func doHTTPRequest(method, url, token string, body []byte) error {
	var reqBody io.Reader
	if body != nil {
		reqBody = bytes.NewReader(body)
	}

	req, err := http.NewRequest(method, url, reqBody)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed to read response: %w", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("HTTP %d: %s", resp.StatusCode, string(respBody))
	}

	fmt.Println(string(respBody))
	return nil
}

// parseFlags extracts --key=value pairs from args.
func parseFlags(args []string) map[string]string {
	flags := make(map[string]string)
	for _, arg := range args {
		if val, ok := strings.CutPrefix(arg, "--"); ok {
			parts := strings.SplitN(val, "=", 2)
			if len(parts) == 2 {
				flags[parts[0]] = parts[1]
			}
		}
	}
	return flags
}

// flagOrEnv returns the flag value if present, otherwise the env var value.
func flagOrEnv(flags map[string]string, flagName, envName string) string {
	if v, ok := flags[flagName]; ok {
		return v
	}
	return os.Getenv(envName)
}

// flagOrEnvDefault returns the flag value, then env var, then a default value.
func flagOrEnvDefault(flags map[string]string, flagName, envName, def string) string {
	if v := flagOrEnv(flags, flagName, envName); v != "" {
		return v
	}
	return def
}
