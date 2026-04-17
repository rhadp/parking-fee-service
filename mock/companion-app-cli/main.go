// companion-app-cli simulates the COMPANION_APP on a mobile device.
// It sends lock/unlock commands and queries command status via CLOUD_GATEWAY REST API.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
)

const defaultGatewayAddr = "http://localhost:8081"

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(args []string) error {
	if len(args) == 0 {
		fmt.Println("companion-app-cli v0.1.0")
		return nil
	}

	subcommand := args[0]
	switch subcommand {
	case "lock":
		return lockCmd(args[1:])
	case "unlock":
		return unlockCmd(args[1:])
	case "status":
		return statusCmd(args[1:])
	default:
		return fmt.Errorf("unknown subcommand %q\nUsage: companion-app-cli <lock|unlock|status> [flags]", subcommand)
	}
}

// parseFlags parses simple --key=value or --key value style flags.
func parseFlags(args []string, known map[string]*string) error {
	i := 0
	for i < len(args) {
		arg := args[i]
		if len(arg) < 2 || arg[0] != '-' {
			return fmt.Errorf("unexpected argument: %q", arg)
		}
		// Strip leading dashes.
		key := arg
		for len(key) > 0 && key[0] == '-' {
			key = key[1:]
		}
		// Handle --key=value.
		val := ""
		hasEq := false
		for j, c := range key {
			if c == '=' {
				val = key[j+1:]
				key = key[:j]
				hasEq = true
				break
			}
		}
		if ptr, ok := known[key]; ok {
			if hasEq {
				*ptr = val
			} else if i+1 < len(args) {
				i++
				*ptr = args[i]
			} else {
				return fmt.Errorf("flag --%s requires a value", key)
			}
		} else {
			return fmt.Errorf("unknown flag: --%s", key)
		}
		i++
	}
	return nil
}

// resolveToken returns the bearer token from flag > env var; errors if neither set.
func resolveToken(flagVal string) (string, error) {
	if flagVal != "" {
		return flagVal, nil
	}
	if v := os.Getenv("CLOUD_GATEWAY_TOKEN"); v != "" {
		return v, nil
	}
	return "", fmt.Errorf("bearer token required: provide --token flag or set CLOUD_GATEWAY_TOKEN environment variable")
}

// resolveGatewayAddr returns the gateway address from flag > env var > default.
func resolveGatewayAddr(flagVal string) string {
	if flagVal != "" {
		return flagVal
	}
	if v := os.Getenv("CLOUD_GATEWAY_ADDR"); v != "" {
		return v
	}
	return defaultGatewayAddr
}

func lockCmd(args []string) error {
	return sendCommand(args, "lock")
}

func unlockCmd(args []string) error {
	return sendCommand(args, "unlock")
}

func sendCommand(args []string, cmdType string) error {
	var vin, token, gatewayAddr string
	flags := map[string]*string{
		"vin":          &vin,
		"token":        &token,
		"gateway-addr": &gatewayAddr,
	}
	if err := parseFlags(args, flags); err != nil {
		return fmt.Errorf("%w\nUsage: companion-app-cli %s --vin=<vin> [--token=<token>] [--gateway-addr=<addr>]", err, cmdType)
	}
	if vin == "" {
		return fmt.Errorf("--vin is required\nUsage: companion-app-cli %s --vin=<vin> [--token=<token>] [--gateway-addr=<addr>]", cmdType)
	}
	tok, err := resolveToken(token)
	if err != nil {
		return err
	}
	addr := resolveGatewayAddr(gatewayAddr)

	body := map[string]any{
		"type":  cmdType,
		"doors": []string{"driver"},
	}
	bodyBytes, _ := json.Marshal(body)

	url := addr + "/vehicles/" + vin + "/commands"
	req, err := http.NewRequest("POST", url, bytes.NewReader(bodyBytes))
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+tok)

	return doHTTPRequest(req)
}

func statusCmd(args []string) error {
	var vin, commandID, token, gatewayAddr string
	flags := map[string]*string{
		"vin":          &vin,
		"command-id":   &commandID,
		"token":        &token,
		"gateway-addr": &gatewayAddr,
	}
	if err := parseFlags(args, flags); err != nil {
		return fmt.Errorf("%w\nUsage: companion-app-cli status --vin=<vin> --command-id=<id> [--token=<token>] [--gateway-addr=<addr>]", err)
	}
	if vin == "" {
		return fmt.Errorf("--vin is required\nUsage: companion-app-cli status --vin=<vin> --command-id=<id>")
	}
	if commandID == "" {
		return fmt.Errorf("--command-id is required\nUsage: companion-app-cli status --vin=<vin> --command-id=<id>")
	}
	tok, err := resolveToken(token)
	if err != nil {
		return err
	}
	addr := resolveGatewayAddr(gatewayAddr)

	url := addr + "/vehicles/" + vin + "/commands/" + commandID
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("Authorization", "Bearer "+tok)

	return doHTTPRequest(req)
}

// doHTTPRequest executes the request, prints response body to stdout on success,
// or prints HTTP status and body to stderr and returns an error on non-2xx.
func doHTTPRequest(req *http.Request) error {
	client := &http.Client{}
	resp, err := client.Do(req)
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	bodyBytes, _ := io.ReadAll(resp.Body)

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("HTTP %d: %s", resp.StatusCode, string(bodyBytes))
	}

	fmt.Print(string(bodyBytes))
	return nil
}
