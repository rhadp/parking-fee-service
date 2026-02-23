package main

import (
	"fmt"
	"os"

	// Import generated proto packages to satisfy TS-01-27 (01-REQ-5.5)
	_ "github.com/rhadp/parking-fee-service/gen/go/commonpb"

	"github.com/spf13/cobra"
)

var (
	gatewayURL string
	vin        string
	token      string
)

func main() {
	if err := rootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}

var rootCmd = &cobra.Command{
	Use:   "companion-app-cli",
	Short: "Mock COMPANION_APP CLI for integration testing",
	Long:  "A mock CLI application that simulates the COMPANION_APP by exposing the same REST interfaces. Used for integration testing without real Android builds.",
}

func init() {
	rootCmd.PersistentFlags().StringVar(&gatewayURL, "gateway-url", "http://localhost:8081", "CLOUD_GATEWAY URL")
	rootCmd.PersistentFlags().StringVar(&vin, "vin", "VIN12345", "Vehicle identification number")
	rootCmd.PersistentFlags().StringVar(&token, "token", "", "Bearer token for authentication")

	rootCmd.AddCommand(lockCmd)
	rootCmd.AddCommand(unlockCmd)
	rootCmd.AddCommand(statusCmd)
}

func notImplemented(cmd *cobra.Command, args []string) {
	fmt.Fprintf(os.Stderr, "not implemented: %s\n", cmd.Name())
	os.Exit(1)
}

var lockCmd = &cobra.Command{
	Use:   "lock",
	Short: "Send lock command via CLOUD_GATEWAY",
	Run:   notImplemented,
}

var unlockCmd = &cobra.Command{
	Use:   "unlock",
	Short: "Send unlock command via CLOUD_GATEWAY",
	Run:   notImplemented,
}

var statusCmd = &cobra.Command{
	Use:   "status",
	Short: "Query vehicle status via CLOUD_GATEWAY",
	Run:   notImplemented,
}
