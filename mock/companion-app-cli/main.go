package main

import (
	"fmt"
	"os"
	"strings"

	"github.com/parking-fee-service/mock/companion-app-cli/cmd"
	"github.com/parking-fee-service/mock/companion-app-cli/internal/config"
)

func main() {
	if len(os.Args) < 2 {
		printUsage()
		os.Exit(0)
	}

	subcmd := os.Args[1]

	if subcmd == "help" || subcmd == "--help" || subcmd == "-h" {
		printUsage()
		os.Exit(0)
	}

	if !cmd.IsValidSubcommand(subcmd) {
		fmt.Fprintf(os.Stderr, "Error: unknown subcommand %q\n", subcmd)
		fmt.Fprintf(os.Stderr, "Valid subcommands: %s\n", strings.Join(cmd.ValidSubcommands, ", "))
		os.Exit(1)
	}

	args := os.Args[2:]

	// Resolve gateway URL: --gateway-url flag overrides env var
	gatewayURL := config.CloudGatewayURL()
	if flagVal := cmd.ExtractFlag(args, "gateway-url"); flagVal != "" {
		gatewayURL = flagVal
	}

	// Resolve bearer token: --token flag overrides CLOUD_GATEWAY_TOKEN env var
	bearerToken := config.BearerToken()
	if flagVal := cmd.ExtractFlag(args, "token"); flagVal != "" {
		bearerToken = flagVal
	}

	if bearerToken == "" {
		fmt.Fprintln(os.Stderr, "Error: no bearer token provided. Set CLOUD_GATEWAY_TOKEN environment variable or use --token flag")
		os.Exit(1)
	}

	err := cmd.Dispatch(subcmd, args, gatewayURL, bearerToken)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}

func printUsage() {
	fmt.Println("Usage: companion-app-cli <subcommand> [flags]")
	fmt.Println()
	fmt.Println("Subcommands:")
	fmt.Println("  lock       Send lock command to vehicle")
	fmt.Println("  unlock     Send unlock command to vehicle")
	fmt.Println("  status     Query command status")
	fmt.Println()
	fmt.Println("Flags:")
	fmt.Println("  --vin=<vin>            Vehicle identification number")
	fmt.Println("  --command-id=<id>      Command ID (for status subcommand)")
	fmt.Println("  --token=<token>        Bearer token for authentication")
	fmt.Println("  --gateway-url=<url>    CLOUD_GATEWAY URL")
	fmt.Println()
	fmt.Println("Environment Variables:")
	fmt.Println("  CLOUD_GATEWAY_URL    (default: http://localhost:8081)")
	fmt.Println("  CLOUD_GATEWAY_TOKEN  Bearer token for authentication")
}
