package main

import (
	"fmt"
	"os"

	// Import generated proto packages to satisfy TS-01-27 (01-REQ-5.5)
	_ "github.com/rhadp/parking-fee-service/gen/go/commonpb"
	_ "github.com/rhadp/parking-fee-service/gen/go/parkingadaptorpb"
	_ "github.com/rhadp/parking-fee-service/gen/go/updateservicepb"

	"github.com/spf13/cobra"
)

var (
	pfsURL      string
	updateAddr  string
	adaptorAddr string
)

func main() {
	if err := rootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}

var rootCmd = &cobra.Command{
	Use:   "parking-app-cli",
	Short: "Mock PARKING_APP CLI for integration testing",
	Long:  "A mock CLI application that simulates the PARKING_APP by exposing the same gRPC/REST interfaces. Used for integration testing without real Android builds.",
}

func init() {
	rootCmd.PersistentFlags().StringVar(&pfsURL, "pfs-url", "http://localhost:8080", "PARKING_FEE_SERVICE URL")
	rootCmd.PersistentFlags().StringVar(&updateAddr, "update-addr", "localhost:50051", "UPDATE_SERVICE gRPC address")
	rootCmd.PersistentFlags().StringVar(&adaptorAddr, "adaptor-addr", "localhost:50052", "PARKING_OPERATOR_ADAPTOR gRPC address")

	rootCmd.AddCommand(lookupCmd)
	rootCmd.AddCommand(installCmd)
	rootCmd.AddCommand(watchCmd)
	rootCmd.AddCommand(listCmd)
	rootCmd.AddCommand(statusCmd)
	rootCmd.AddCommand(startSessionCmd)
	rootCmd.AddCommand(stopSessionCmd)
	rootCmd.AddCommand(getStatusCmd)
	rootCmd.AddCommand(getRateCmd)
}

func notImplemented(cmd *cobra.Command, args []string) {
	fmt.Fprintf(os.Stderr, "not implemented: %s\n", cmd.Name())
	os.Exit(1)
}

var lookupCmd = &cobra.Command{
	Use:   "lookup",
	Short: "Query PARKING_FEE_SERVICE for operators by location",
	Run:   notImplemented,
}

var installCmd = &cobra.Command{
	Use:   "install",
	Short: "Request adapter installation via UPDATE_SERVICE",
	Run:   notImplemented,
}

var watchCmd = &cobra.Command{
	Use:   "watch",
	Short: "Watch adapter state changes via UPDATE_SERVICE",
	Run:   notImplemented,
}

var listCmd = &cobra.Command{
	Use:   "list",
	Short: "List installed adapters via UPDATE_SERVICE",
	Run:   notImplemented,
}

var statusCmd = &cobra.Command{
	Use:   "status",
	Short: "Get adapter status via UPDATE_SERVICE",
	Run:   notImplemented,
}

var startSessionCmd = &cobra.Command{
	Use:   "start-session",
	Short: "Start a parking session via PARKING_OPERATOR_ADAPTOR",
	Run:   notImplemented,
}

var stopSessionCmd = &cobra.Command{
	Use:   "stop-session",
	Short: "Stop a parking session via PARKING_OPERATOR_ADAPTOR",
	Run:   notImplemented,
}

var getStatusCmd = &cobra.Command{
	Use:   "get-status",
	Short: "Get parking session status",
	Run:   notImplemented,
}

var getRateCmd = &cobra.Command{
	Use:   "get-rate",
	Short: "Get parking rate for a zone",
	Run:   notImplemented,
}
