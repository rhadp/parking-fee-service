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
		os.Exit(1)
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

	gatewayURL := config.CloudGatewayURL()
	bearerToken := config.BearerToken()

	if bearerToken == "" {
		fmt.Fprintln(os.Stderr, "Warning: BEARER_TOKEN not set, proceeding without auth header")
	}

	args := os.Args[2:]
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
	fmt.Println("  status     Query vehicle status")
	fmt.Println()
	fmt.Println("Environment Variables:")
	fmt.Println("  CLOUD_GATEWAY_URL  (default: http://localhost:8081)")
	fmt.Println("  BEARER_TOKEN       (default: empty)")
}
