package main

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "Usage: companion-app-cli <lock|unlock|status> [flags]")
		os.Exit(1)
	}

	subcmd := os.Args[1]

	switch subcmd {
	case "lock":
		runLockUnlock("lock")
	case "unlock":
		runLockUnlock("unlock")
	case "status":
		runStatus()
	default:
		fmt.Fprintf(os.Stderr, "unknown subcommand: %s\n", subcmd)
		os.Exit(1)
	}
}

// runLockUnlock handles both lock and unlock subcommands.
func runLockUnlock(cmdType string) {
	vin := getFlag("--vin")
	if vin == "" {
		fmt.Fprintln(os.Stderr, "error: --vin is required")
		os.Exit(1)
	}

	token := resolveToken()
	addr := resolveGatewayAddr()

	url := fmt.Sprintf("%s/vehicles/%s/commands", addr, vin)
	body := fmt.Sprintf(`{"type":"%s","doors":["driver"]}`, cmdType)

	req, err := http.NewRequest("POST", url, strings.NewReader(body))
	if err != nil {
		fmt.Fprintf(os.Stderr, "error creating request: %v\n", err)
		os.Exit(1)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
	defer resp.Body.Close()

	respBody, _ := io.ReadAll(resp.Body)

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		fmt.Fprintf(os.Stderr, "error: HTTP %d: %s\n", resp.StatusCode, string(respBody))
		os.Exit(1)
	}

	fmt.Print(string(respBody))
}

// runStatus handles the status subcommand.
func runStatus() {
	vin := getFlag("--vin")
	if vin == "" {
		fmt.Fprintln(os.Stderr, "error: --vin is required")
		os.Exit(1)
	}

	commandID := getFlag("--command-id")
	if commandID == "" {
		fmt.Fprintln(os.Stderr, "error: --command-id is required")
		os.Exit(1)
	}

	token := resolveToken()
	addr := resolveGatewayAddr()

	url := fmt.Sprintf("%s/vehicles/%s/commands/%s", addr, vin, commandID)

	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error creating request: %v\n", err)
		os.Exit(1)
	}
	req.Header.Set("Authorization", "Bearer "+token)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
	defer resp.Body.Close()

	respBody, _ := io.ReadAll(resp.Body)

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		fmt.Fprintf(os.Stderr, "error: HTTP %d: %s\n", resp.StatusCode, string(respBody))
		os.Exit(1)
	}

	fmt.Print(string(respBody))
}

// resolveToken returns the bearer token from --token flag or CLOUD_GATEWAY_TOKEN env var.
func resolveToken() string {
	if token := getFlag("--token"); token != "" {
		return token
	}
	if token := os.Getenv("CLOUD_GATEWAY_TOKEN"); token != "" {
		return token
	}
	fmt.Fprintln(os.Stderr, "error: no token provided; use --token flag or set CLOUD_GATEWAY_TOKEN environment variable")
	os.Exit(1)
	return ""
}

// resolveGatewayAddr returns the CLOUD_GATEWAY address from --gateway-addr flag
// or CLOUD_GATEWAY_ADDR env var, defaulting to http://localhost:8081.
func resolveGatewayAddr() string {
	if addr := getFlag("--gateway-addr"); addr != "" {
		return addr
	}
	if addr := os.Getenv("CLOUD_GATEWAY_ADDR"); addr != "" {
		return addr
	}
	return "http://localhost:8081"
}

// getFlag extracts a --key=value style flag from os.Args.
func getFlag(name string) string {
	prefix := name + "="
	for _, arg := range os.Args[2:] {
		if val, ok := strings.CutPrefix(arg, prefix); ok {
			return val
		}
	}
	return ""
}
