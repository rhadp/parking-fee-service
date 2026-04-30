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
	if len(os.Args) < 2 {
		fmt.Fprintf(os.Stderr, "Usage: %s <lock|unlock|status> [flags]\n", os.Args[0])
		os.Exit(1)
	}

	subcommand := os.Args[1]

	switch subcommand {
	case "lock":
		runLockUnlock("lock")
	case "unlock":
		runLockUnlock("unlock")
	case "status":
		runStatus()
	default:
		fmt.Fprintf(os.Stderr, "Error: unknown subcommand %q\n", subcommand)
		fmt.Fprintf(os.Stderr, "Usage: %s <lock|unlock|status> [flags]\n", os.Args[0])
		os.Exit(1)
	}
}

// runLockUnlock handles both lock and unlock subcommands.
func runLockUnlock(cmdType string) {
	vin := resolveFlag("--vin", "")
	if vin == "" {
		fmt.Fprintf(os.Stderr, "Error: --vin is required\n")
		os.Exit(1)
	}

	token := resolveFlag("--token", "CLOUD_GATEWAY_TOKEN")
	if token == "" {
		fmt.Fprintf(os.Stderr, "Error: bearer token is required (--token flag or CLOUD_GATEWAY_TOKEN env var)\n")
		os.Exit(1)
	}

	gatewayAddr := resolveFlag("--gateway-addr", "CLOUD_GATEWAY_ADDR")
	if gatewayAddr == "" {
		gatewayAddr = "http://localhost:8081"
	}

	body := map[string]any{
		"type":  cmdType,
		"doors": []string{"driver"},
	}
	bodyBytes, err := json.Marshal(body)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: failed to marshal request body: %v\n", err)
		os.Exit(1)
	}

	url := fmt.Sprintf("%s/vehicles/%s/commands", gatewayAddr, vin)
	req, err := http.NewRequest("POST", url, bytes.NewReader(bodyBytes))
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: failed to create request: %v\n", err)
		os.Exit(1)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: request failed: %v\n", err)
		os.Exit(1)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: failed to read response: %v\n", err)
		os.Exit(1)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		fmt.Fprintf(os.Stderr, "Error: HTTP %d: %s\n", resp.StatusCode, string(respBody))
		os.Exit(1)
	}

	fmt.Println(string(respBody))
}

// runStatus handles the status subcommand.
func runStatus() {
	vin := resolveFlag("--vin", "")
	if vin == "" {
		fmt.Fprintf(os.Stderr, "Error: --vin is required\n")
		os.Exit(1)
	}

	commandID := resolveFlag("--command-id", "")
	if commandID == "" {
		fmt.Fprintf(os.Stderr, "Error: --command-id is required\n")
		os.Exit(1)
	}

	token := resolveFlag("--token", "CLOUD_GATEWAY_TOKEN")
	if token == "" {
		fmt.Fprintf(os.Stderr, "Error: bearer token is required (--token flag or CLOUD_GATEWAY_TOKEN env var)\n")
		os.Exit(1)
	}

	gatewayAddr := resolveFlag("--gateway-addr", "CLOUD_GATEWAY_ADDR")
	if gatewayAddr == "" {
		gatewayAddr = "http://localhost:8081"
	}

	url := fmt.Sprintf("%s/vehicles/%s/commands/%s", gatewayAddr, vin, commandID)
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: failed to create request: %v\n", err)
		os.Exit(1)
	}
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: request failed: %v\n", err)
		os.Exit(1)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: failed to read response: %v\n", err)
		os.Exit(1)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		fmt.Fprintf(os.Stderr, "Error: HTTP %d: %s\n", resp.StatusCode, string(respBody))
		os.Exit(1)
	}

	fmt.Println(string(respBody))
}

// resolveFlag extracts a --flag=value from os.Args, falling back to env var.
func resolveFlag(flag, envVar string) string {
	prefix := flag + "="
	for _, arg := range os.Args[2:] {
		if v, ok := strings.CutPrefix(arg, prefix); ok {
			return v
		}
	}
	if envVar != "" {
		return os.Getenv(envVar)
	}
	return ""
}
