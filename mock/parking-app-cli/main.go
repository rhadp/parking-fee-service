package main

import (
	"fmt"
	"os"
	"strings"

	"github.com/parking-fee-service/mock/parking-app-cli/cmd"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/config"
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
	err := cmd.Dispatch(subcmd, args,
		config.ParkingFeeServiceURL(),
		config.UpdateServiceAddr(),
		config.ParkingAdaptorAddr(),
	)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}

func printUsage() {
	fmt.Println("Usage: parking-app-cli <subcommand> [flags]")
	fmt.Println()
	fmt.Println("Subcommands:")
	fmt.Println("  lookup          Query operators by location")
	fmt.Println("  adapter-info    Get adapter metadata for an operator")
	fmt.Println("  install         Install an adapter via UPDATE_SERVICE")
	fmt.Println("  watch           Watch adapter state changes (streaming)")
	fmt.Println("  list            List installed adapters")
	fmt.Println("  remove          Remove an installed adapter")
	fmt.Println("  status          Get adapter status")
	fmt.Println("  start-session   Start a parking session")
	fmt.Println("  stop-session    Stop a parking session")
	fmt.Println()
	fmt.Println("Environment Variables:")
	fmt.Println("  PARKING_FEE_SERVICE_URL  (default: http://localhost:8080)")
	fmt.Println("  UPDATE_SERVICE_ADDR      (default: localhost:50052)")
	fmt.Println("  ADAPTOR_ADDR             (default: localhost:50053)")
}
