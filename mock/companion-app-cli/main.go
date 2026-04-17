// companion-app-cli simulates the COMPANION_APP sending lock/unlock commands
// to CLOUD_GATEWAY via REST. It supports lock, unlock, and status subcommands.
//
// Usage:
//
//	companion-app-cli lock   --vin=<vin> [--token=<token>] [--gateway-addr=<addr>]
//	companion-app-cli unlock --vin=<vin> [--token=<token>] [--gateway-addr=<addr>]
//	companion-app-cli status --vin=<vin> --command-id=<id> [--token=<token>] [--gateway-addr=<addr>]
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

const defaultGatewayAddr = "http://localhost:8081"

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: companion-app-cli <lock|unlock|status> [flags]")
		os.Exit(1)
	}

	subcommand := os.Args[1]
	args := os.Args[2:]

	switch subcommand {
	case "lock":
		runLockUnlock("lock", args)
	case "unlock":
		runLockUnlock("unlock", args)
	case "status":
		runStatus(args)
	default:
		fmt.Fprintf(os.Stderr, "unknown subcommand: %s\n", subcommand)
		fmt.Fprintln(os.Stderr, "usage: companion-app-cli <lock|unlock|status> [flags]")
		os.Exit(1)
	}
}

// resolveToken returns the bearer token from the flag value or CLOUD_GATEWAY_TOKEN env var.
// Exits with code 1 if no token is available.
func resolveToken(flagVal string) string {
	if flagVal != "" {
		return flagVal
	}
	t := os.Getenv("CLOUD_GATEWAY_TOKEN")
	if t == "" {
		fmt.Fprintln(os.Stderr, "error: bearer token required — use --token or CLOUD_GATEWAY_TOKEN env var")
		os.Exit(1)
	}
	return t
}

// resolveGatewayAddr returns the gateway address from the flag value or CLOUD_GATEWAY_ADDR env var.
func resolveGatewayAddr(flagVal string) string {
	if flagVal != "" {
		return flagVal
	}
	if addr := os.Getenv("CLOUD_GATEWAY_ADDR"); addr != "" {
		return addr
	}
	return defaultGatewayAddr
}

// doRequest performs an HTTP request with the given method, url, bearer token, and optional body.
// It prints the response JSON to stdout and exits 0 on success, or prints error to stderr and exits 1.
func doRequest(method, url, token string, body []byte) {
	var bodyReader io.Reader
	if body != nil {
		bodyReader = bytes.NewReader(body)
	}

	req, err := http.NewRequest(method, url, bodyReader)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error creating request: %v\n", err)
		os.Exit(1)
	}
	req.Header.Set("Authorization", "Bearer "+token)
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error reading response: %v\n", err)
		os.Exit(1)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		fmt.Fprintf(os.Stderr, "HTTP %d: %s\n", resp.StatusCode, strings.TrimSpace(string(respBody)))
		os.Exit(1)
	}

	// Pretty-print JSON if possible, else print raw.
	var pretty bytes.Buffer
	if json.Indent(&pretty, respBody, "", "  ") == nil {
		fmt.Println(pretty.String())
	} else {
		fmt.Println(string(respBody))
	}
}

// runLockUnlock handles the lock and unlock subcommands.
func runLockUnlock(cmdType string, args []string) {
	fs := flag.NewFlagSet(cmdType, flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	vin := fs.String("vin", "", "vehicle identification number (required)")
	token := fs.String("token", "", "bearer token (or CLOUD_GATEWAY_TOKEN env var)")
	gatewayAddr := fs.String("gateway-addr", "", "CLOUD_GATEWAY address (default http://localhost:8081)")

	if err := fs.Parse(args); err != nil {
		os.Exit(1)
	}

	if *vin == "" {
		fmt.Fprintf(os.Stderr, "error: --vin is required\n")
		fs.Usage()
		os.Exit(1)
	}

	tok := resolveToken(*token)
	gw := resolveGatewayAddr(*gatewayAddr)

	body, err := json.Marshal(map[string]any{
		"type":  cmdType,
		"doors": []string{"driver"},
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "error marshaling request: %v\n", err)
		os.Exit(1)
	}

	url := strings.TrimRight(gw, "/") + "/vehicles/" + *vin + "/commands"
	doRequest("POST", url, tok, body)
}

// runStatus handles the status subcommand.
func runStatus(args []string) {
	fs := flag.NewFlagSet("status", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	vin := fs.String("vin", "", "vehicle identification number (required)")
	commandID := fs.String("command-id", "", "command ID to query (required)")
	token := fs.String("token", "", "bearer token (or CLOUD_GATEWAY_TOKEN env var)")
	gatewayAddr := fs.String("gateway-addr", "", "CLOUD_GATEWAY address (default http://localhost:8081)")

	if err := fs.Parse(args); err != nil {
		os.Exit(1)
	}

	if *vin == "" {
		fmt.Fprintf(os.Stderr, "error: --vin is required\n")
		fs.Usage()
		os.Exit(1)
	}
	if *commandID == "" {
		fmt.Fprintf(os.Stderr, "error: --command-id is required\n")
		fs.Usage()
		os.Exit(1)
	}

	tok := resolveToken(*token)
	gw := resolveGatewayAddr(*gatewayAddr)

	url := strings.TrimRight(gw, "/") + "/vehicles/" + *vin + "/commands/" + *commandID
	doRequest("GET", url, tok, nil)
}
