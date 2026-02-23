package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"time"

	"github.com/rhadp/parking-fee-service/gen/go/commonpb"
	"github.com/rhadp/parking-fee-service/gen/go/parkingadaptorpb"
	"github.com/rhadp/parking-fee-service/gen/go/updateservicepb"

	"github.com/spf13/cobra"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

var (
	pfsURL      string
	pfsToken    string
	updateAddr  string
	adaptorAddr string
)

func main() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintln(os.Stderr, err)
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
	rootCmd.PersistentFlags().StringVar(&pfsToken, "token", "demo-token-1", "Bearer token for PARKING_FEE_SERVICE")
	rootCmd.PersistentFlags().StringVar(&updateAddr, "update-addr", "localhost:50051", "UPDATE_SERVICE gRPC address")
	rootCmd.PersistentFlags().StringVar(&adaptorAddr, "adaptor-addr", "localhost:50052", "PARKING_OPERATOR_ADAPTOR gRPC address")

	rootCmd.AddCommand(lookupCmd)
	rootCmd.AddCommand(adapterCmd)
	rootCmd.AddCommand(installCmd)
	rootCmd.AddCommand(watchCmd)
	rootCmd.AddCommand(listCmd)
	rootCmd.AddCommand(statusCmd)
	rootCmd.AddCommand(startSessionCmd)
	rootCmd.AddCommand(stopSessionCmd)
	rootCmd.AddCommand(getStatusCmd)
	rootCmd.AddCommand(getRateCmd)
}

// dialGRPC creates a gRPC client connection to the given address.
func dialGRPC(addr string) (*grpc.ClientConn, error) {
	conn, err := grpc.NewClient(addr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to %s: %w", addr, err)
	}
	return conn, nil
}

// --- lookup command (PARKING_FEE_SERVICE — REST) ---

var lookupLat float64
var lookupLon float64

var lookupCmd = &cobra.Command{
	Use:           "lookup",
	Short:         "Query PARKING_FEE_SERVICE for operators by location",
	SilenceUsage:  true,
	SilenceErrors: true,
	RunE:          runLookup,
}

func init() {
	lookupCmd.Flags().Float64Var(&lookupLat, "lat", 0, "Latitude of the vehicle")
	lookupCmd.Flags().Float64Var(&lookupLon, "lon", 0, "Longitude of the vehicle")
}

// operatorsResponse matches the JSON structure returned by
// GET /operators?lat=&lon= on the PARKING_FEE_SERVICE.
type operatorsResponse struct {
	Operators []operatorResult `json:"operators"`
}

type operatorResult struct {
	OperatorID string     `json:"operator_id"`
	Name       string     `json:"name"`
	Zone       zoneResult `json:"zone"`
	Rate       rateResult `json:"rate"`
}

type zoneResult struct {
	ZoneID string `json:"zone_id"`
	Name   string `json:"name"`
}

type rateResult struct {
	AmountPerHour float64 `json:"amount_per_hour"`
	Currency      string  `json:"currency"`
}

// pfsErrorResponse matches the JSON error structure returned by
// PARKING_FEE_SERVICE on 4xx/5xx responses.
type pfsErrorResponse struct {
	Error string `json:"error"`
}

// runLookup executes the lookup command against the PARKING_FEE_SERVICE.
func runLookup(cmd *cobra.Command, args []string) error {
	url := fmt.Sprintf("%s/operators?lat=%g&lon=%g", pfsURL, lookupLat, lookupLon)

	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("Authorization", "Bearer "+pfsToken)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return fmt.Errorf("request failed (target: %s): %w", pfsURL, err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed to read response: %w", err)
	}

	if resp.StatusCode != http.StatusOK {
		var errResp pfsErrorResponse
		if json.Unmarshal(body, &errResp) == nil && errResp.Error != "" {
			return fmt.Errorf("lookup failed: %s", errResp.Error)
		}
		return fmt.Errorf("lookup failed: HTTP %d", resp.StatusCode)
	}

	var opsResp operatorsResponse
	if err := json.Unmarshal(body, &opsResp); err != nil {
		return fmt.Errorf("failed to parse response: %w", err)
	}

	fmt.Fprintf(cmd.OutOrStdout(), "Found %d operator(s):\n", len(opsResp.Operators))
	for i, op := range opsResp.Operators {
		fmt.Fprintf(cmd.OutOrStdout(), "  [%d] %s (%s)\n", i+1, op.Name, op.OperatorID)
		fmt.Fprintf(cmd.OutOrStdout(), "      Zone: %s (%s)\n", op.Zone.Name, op.Zone.ZoneID)
		fmt.Fprintf(cmd.OutOrStdout(), "      Rate: %.2f %s/hour\n", op.Rate.AmountPerHour, op.Rate.Currency)
	}
	return nil
}

// --- adapter command (PARKING_FEE_SERVICE — REST) ---

var adapterOperatorID string

var adapterCmd = &cobra.Command{
	Use:           "adapter",
	Short:         "Retrieve adapter metadata for an operator via PARKING_FEE_SERVICE",
	SilenceUsage:  true,
	SilenceErrors: true,
	RunE:          runAdapter,
}

func init() {
	adapterCmd.Flags().StringVar(&adapterOperatorID, "operator-id", "", "Operator ID to retrieve adapter metadata for")
}

// adapterMetadataResponse matches the JSON structure returned by
// GET /operators/{id}/adapter on the PARKING_FEE_SERVICE.
type adapterMetadataResponse struct {
	OperatorID     string `json:"operator_id"`
	ImageRef       string `json:"image_ref"`
	ChecksumSHA256 string `json:"checksum_sha256"`
	Version        string `json:"version"`
}

// runAdapter executes the adapter command against the PARKING_FEE_SERVICE.
func runAdapter(cmd *cobra.Command, args []string) error {
	if adapterOperatorID == "" {
		return fmt.Errorf("--operator-id is required")
	}

	url := fmt.Sprintf("%s/operators/%s/adapter", pfsURL, adapterOperatorID)

	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("Authorization", "Bearer "+pfsToken)

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return fmt.Errorf("request failed (target: %s): %w", pfsURL, err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed to read response: %w", err)
	}

	if resp.StatusCode != http.StatusOK {
		var errResp pfsErrorResponse
		if json.Unmarshal(body, &errResp) == nil && errResp.Error != "" {
			return fmt.Errorf("adapter lookup failed: %s", errResp.Error)
		}
		return fmt.Errorf("adapter lookup failed: HTTP %d", resp.StatusCode)
	}

	var adpResp adapterMetadataResponse
	if err := json.Unmarshal(body, &adpResp); err != nil {
		return fmt.Errorf("failed to parse response: %w", err)
	}

	fmt.Fprintf(cmd.OutOrStdout(), "Adapter metadata for operator %s:\n", adpResp.OperatorID)
	fmt.Fprintf(cmd.OutOrStdout(), "  Image: %s\n", adpResp.ImageRef)
	fmt.Fprintf(cmd.OutOrStdout(), "  Checksum: %s\n", adpResp.ChecksumSHA256)
	fmt.Fprintf(cmd.OutOrStdout(), "  Version: %s\n", adpResp.Version)
	return nil
}

// --- install command (UPDATE_SERVICE) ---
// 04-REQ-9.1

var installImageRef string
var installChecksum string

var installCmd = &cobra.Command{
	Use:           "install",
	Short:         "Request adapter installation via UPDATE_SERVICE",
	SilenceUsage:  true,
	SilenceErrors: true,
	RunE: func(cmd *cobra.Command, args []string) error {
		conn, err := dialGRPC(updateAddr)
		if err != nil {
			return err
		}
		defer conn.Close()

		client := updateservicepb.NewUpdateServiceClient(conn)
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()

		resp, err := client.InstallAdapter(ctx, &updateservicepb.InstallAdapterRequest{
			ImageRef:      installImageRef,
			ChecksumSha256: installChecksum,
		})
		if err != nil {
			return fmt.Errorf("install failed (target: %s): %w", updateAddr, err)
		}

		fmt.Printf("job_id: %s\n", resp.GetJobId())
		fmt.Printf("adapter_id: %s\n", resp.GetAdapterId())
		fmt.Printf("state: %s\n", resp.GetState().String())
		return nil
	},
}

func init() {
	installCmd.Flags().StringVar(&installImageRef, "image-ref", "", "OCI image reference")
	installCmd.Flags().StringVar(&installChecksum, "checksum", "", "SHA-256 checksum of the image")
}

// --- watch command (UPDATE_SERVICE) ---
// 04-REQ-9.2

var watchCmd = &cobra.Command{
	Use:           "watch",
	Short:         "Watch adapter state changes via UPDATE_SERVICE",
	SilenceUsage:  true,
	SilenceErrors: true,
	RunE: func(cmd *cobra.Command, args []string) error {
		conn, err := dialGRPC(updateAddr)
		if err != nil {
			return err
		}
		defer conn.Close()

		client := updateservicepb.NewUpdateServiceClient(conn)
		ctx := context.Background()

		stream, err := client.WatchAdapterStates(ctx, &updateservicepb.WatchAdapterStatesRequest{})
		if err != nil {
			return fmt.Errorf("watch failed (target: %s): %w", updateAddr, err)
		}

		for {
			event, err := stream.Recv()
			if err == io.EOF {
				return nil
			}
			if err != nil {
				return fmt.Errorf("watch stream error: %w", err)
			}
			fmt.Printf("adapter_id: %s  old_state: %s  new_state: %s  timestamp: %d\n",
				event.GetAdapterId(),
				event.GetOldState().String(),
				event.GetNewState().String(),
				event.GetTimestamp(),
			)
		}
	},
}

// --- list command (UPDATE_SERVICE) ---
// 04-REQ-9.3

var listCmd = &cobra.Command{
	Use:           "list",
	Short:         "List installed adapters via UPDATE_SERVICE",
	SilenceUsage:  true,
	SilenceErrors: true,
	RunE: func(cmd *cobra.Command, args []string) error {
		conn, err := dialGRPC(updateAddr)
		if err != nil {
			return err
		}
		defer conn.Close()

		client := updateservicepb.NewUpdateServiceClient(conn)
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()

		resp, err := client.ListAdapters(ctx, &updateservicepb.ListAdaptersRequest{})
		if err != nil {
			return fmt.Errorf("list failed (target: %s): %w", updateAddr, err)
		}

		if len(resp.GetAdapters()) == 0 {
			fmt.Println("No adapters found.")
			return nil
		}

		fmt.Printf("%-38s  %-40s  %s\n", "ID", "IMAGE", "STATE")
		fmt.Printf("%-38s  %-40s  %s\n", "---", "-----", "-----")
		for _, a := range resp.GetAdapters() {
			fmt.Printf("%-38s  %-40s  %s\n",
				a.GetAdapterId(),
				a.GetImageRef(),
				a.GetState().String(),
			)
		}
		return nil
	},
}

// --- status command (UPDATE_SERVICE adapter status) ---

var statusAdapterID string

var statusCmd = &cobra.Command{
	Use:           "status",
	Short:         "Get adapter status via UPDATE_SERVICE",
	SilenceUsage:  true,
	SilenceErrors: true,
	RunE: func(cmd *cobra.Command, args []string) error {
		conn, err := dialGRPC(updateAddr)
		if err != nil {
			return err
		}
		defer conn.Close()

		client := updateservicepb.NewUpdateServiceClient(conn)
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()

		resp, err := client.GetAdapterStatus(ctx, &updateservicepb.GetAdapterStatusRequest{
			AdapterId: statusAdapterID,
		})
		if err != nil {
			return fmt.Errorf("status failed (target: %s): %w", updateAddr, err)
		}

		a := resp.GetAdapter()
		fmt.Printf("adapter_id: %s\n", a.GetAdapterId())
		fmt.Printf("image_ref: %s\n", a.GetImageRef())
		fmt.Printf("state: %s\n", a.GetState().String())
		return nil
	},
}

func init() {
	statusCmd.Flags().StringVar(&statusAdapterID, "adapter-id", "", "Adapter ID to query")
}

// --- start-session command (PARKING_OPERATOR_ADAPTOR) ---
// 04-REQ-9.4

var startVehicleID string
var startZoneID string

var startSessionCmd = &cobra.Command{
	Use:           "start-session",
	Short:         "Start a parking session via PARKING_OPERATOR_ADAPTOR",
	SilenceUsage:  true,
	SilenceErrors: true,
	RunE: func(cmd *cobra.Command, args []string) error {
		conn, err := dialGRPC(adaptorAddr)
		if err != nil {
			return err
		}
		defer conn.Close()

		client := parkingadaptorpb.NewParkingAdaptorClient(conn)
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()

		resp, err := client.StartSession(ctx, &parkingadaptorpb.StartSessionRequest{
			VehicleId: startVehicleID,
			ZoneId:    startZoneID,
		})
		if err != nil {
			return fmt.Errorf("start-session failed (target: %s): %w", adaptorAddr, err)
		}

		fmt.Printf("session_id: %s\n", resp.GetSessionId())
		fmt.Printf("status: %s\n", resp.GetStatus())
		return nil
	},
}

func init() {
	startSessionCmd.Flags().StringVar(&startVehicleID, "vehicle-id", "", "Vehicle ID")
	startSessionCmd.Flags().StringVar(&startZoneID, "zone-id", "", "Zone ID")
}

// --- stop-session command (PARKING_OPERATOR_ADAPTOR) ---
// 04-REQ-9.5

var stopSessionID string

var stopSessionCmd = &cobra.Command{
	Use:           "stop-session",
	Short:         "Stop a parking session via PARKING_OPERATOR_ADAPTOR",
	SilenceUsage:  true,
	SilenceErrors: true,
	RunE: func(cmd *cobra.Command, args []string) error {
		conn, err := dialGRPC(adaptorAddr)
		if err != nil {
			return err
		}
		defer conn.Close()

		client := parkingadaptorpb.NewParkingAdaptorClient(conn)
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()

		resp, err := client.StopSession(ctx, &parkingadaptorpb.StopSessionRequest{
			SessionId: stopSessionID,
		})
		if err != nil {
			return fmt.Errorf("stop-session failed (target: %s): %w", adaptorAddr, err)
		}

		fmt.Printf("session_id: %s\n", resp.GetSessionId())
		fmt.Printf("fee: %.2f\n", resp.GetTotalFee())
		fmt.Printf("duration: %d\n", resp.GetDurationSeconds())
		fmt.Printf("currency: %s\n", resp.GetCurrency())
		return nil
	},
}

func init() {
	stopSessionCmd.Flags().StringVar(&stopSessionID, "session-id", "", "Session ID to stop")
}

// --- get-status command (PARKING_OPERATOR_ADAPTOR session status) ---

var getStatusSessionID string

var getStatusCmd = &cobra.Command{
	Use:           "get-status",
	Short:         "Get parking session status",
	SilenceUsage:  true,
	SilenceErrors: true,
	RunE: func(cmd *cobra.Command, args []string) error {
		conn, err := dialGRPC(adaptorAddr)
		if err != nil {
			return err
		}
		defer conn.Close()

		client := parkingadaptorpb.NewParkingAdaptorClient(conn)
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()

		resp, err := client.GetStatus(ctx, &parkingadaptorpb.GetStatusRequest{
			SessionId: getStatusSessionID,
		})
		if err != nil {
			return fmt.Errorf("get-status failed (target: %s): %w", adaptorAddr, err)
		}

		fmt.Printf("session_id: %s\n", resp.GetSessionId())
		fmt.Printf("active: %v\n", resp.GetActive())
		fmt.Printf("start_time: %d\n", resp.GetStartTime())
		fmt.Printf("current_fee: %.2f\n", resp.GetCurrentFee())
		fmt.Printf("currency: %s\n", resp.GetCurrency())
		return nil
	},
}

func init() {
	getStatusCmd.Flags().StringVar(&getStatusSessionID, "session-id", "", "Session ID to query")
}

// --- get-rate command (PARKING_OPERATOR_ADAPTOR) ---

var getRateZoneID string

var getRateCmd = &cobra.Command{
	Use:           "get-rate",
	Short:         "Get parking rate for a zone",
	SilenceUsage:  true,
	SilenceErrors: true,
	RunE: func(cmd *cobra.Command, args []string) error {
		conn, err := dialGRPC(adaptorAddr)
		if err != nil {
			return err
		}
		defer conn.Close()

		client := parkingadaptorpb.NewParkingAdaptorClient(conn)
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()

		resp, err := client.GetRate(ctx, &parkingadaptorpb.GetRateRequest{
			ZoneId: getRateZoneID,
		})
		if err != nil {
			return fmt.Errorf("get-rate failed (target: %s): %w", adaptorAddr, err)
		}

		fmt.Printf("zone_name: %s\n", resp.GetZoneName())
		fmt.Printf("rate_per_hour: %.2f\n", resp.GetRatePerHour())
		fmt.Printf("currency: %s\n", resp.GetCurrency())
		return nil
	},
}

func init() {
	getRateCmd.Flags().StringVar(&getRateZoneID, "zone-id", "", "Zone ID to query rate for")
}

// Ensure generated proto packages are importable (TS-01-27 / 01-REQ-5.5).
var (
	_ = commonpb.AdapterState_ADAPTER_STATE_UNKNOWN
)
