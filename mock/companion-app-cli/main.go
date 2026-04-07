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
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(args []string) error {
	if len(args) == 0 {
		return fmt.Errorf("usage: companion-app-cli <lock|unlock|status> [flags]")
	}

	subcommand := args[0]
	flags := parseFlags(args[1:])

	// Resolve token: --token flag > CLOUD_GATEWAY_TOKEN env var
	token := flags["token"]
	if token == "" {
		token = os.Getenv("CLOUD_GATEWAY_TOKEN")
	}
	if token == "" {
		return fmt.Errorf("error: no bearer token provided; use --token=<token> or set CLOUD_GATEWAY_TOKEN")
	}

	// Resolve gateway address
	gatewayAddr := flags["gateway-addr"]
	if gatewayAddr == "" {
		gatewayAddr = os.Getenv("CLOUD_GATEWAY_ADDR")
	}
	if gatewayAddr == "" {
		gatewayAddr = "http://localhost:8081"
	}

	switch subcommand {
	case "lock":
		return doLockUnlock(gatewayAddr, token, flags, "lock")
	case "unlock":
		return doLockUnlock(gatewayAddr, token, flags, "unlock")
	case "status":
		return doStatus(gatewayAddr, token, flags)
	default:
		return fmt.Errorf("unknown subcommand: %s\nusage: companion-app-cli <lock|unlock|status> [flags]", subcommand)
	}
}

func doLockUnlock(gatewayAddr, token string, flags map[string]string, cmdType string) error {
	vin := flags["vin"]
	if vin == "" {
		return fmt.Errorf("error: --vin is required")
	}

	body := map[string]any{
		"type":  cmdType,
		"doors": []string{"driver"},
	}
	bodyBytes, _ := json.Marshal(body)

	url := fmt.Sprintf("%s/vehicles/%s/commands", gatewayAddr, vin)
	req, err := http.NewRequest("POST", url, bytes.NewReader(bodyBytes))
	if err != nil {
		return fmt.Errorf("error creating request: %v", err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)

	return doHTTPRequest(req)
}

func doStatus(gatewayAddr, token string, flags map[string]string) error {
	vin := flags["vin"]
	if vin == "" {
		return fmt.Errorf("error: --vin is required")
	}
	commandID := flags["command-id"]
	if commandID == "" {
		return fmt.Errorf("error: --command-id is required")
	}

	url := fmt.Sprintf("%s/vehicles/%s/commands/%s", gatewayAddr, vin, commandID)
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return fmt.Errorf("error creating request: %v", err)
	}
	req.Header.Set("Authorization", "Bearer "+token)

	return doHTTPRequest(req)
}

func doHTTPRequest(req *http.Request) error {
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return fmt.Errorf("error: %v", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("error reading response: %v", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("HTTP %d: %s", resp.StatusCode, string(respBody))
	}

	fmt.Print(string(respBody))
	return nil
}

// parseFlags parses --key=value flags into a map.
func parseFlags(args []string) map[string]string {
	flags := make(map[string]string)
	for _, arg := range args {
		if strings.HasPrefix(arg, "--") {
			kv := strings.TrimPrefix(arg, "--")
			parts := strings.SplitN(kv, "=", 2)
			if len(parts) == 2 {
				flags[parts[0]] = parts[1]
			}
		}
	}
	return flags
}
