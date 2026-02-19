// Package main implements the parking-app-cli mock application.
//
// This CLI simulates the PARKING_APP Android application by invoking gRPC
// calls against UpdateService and ParkingAdapter, and REST calls against
// PARKING_FEE_SERVICE. It uses the same .proto definitions and generated Go
// stubs as the real application will, and standard net/http for REST calls.
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"syscall"
	"time"

	commonpb "github.com/rhadp/parking-fee-service/proto/gen/go/common"
	adapterpb "github.com/rhadp/parking-fee-service/proto/gen/go/services/adapter"
	updatepb "github.com/rhadp/parking-fee-service/proto/gen/go/services/update"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// Global configuration populated from flags / environment.
var (
	updateServiceAddr      string
	adapterAddr            string
	parkingFeeServiceAddr  string
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}

// run parses arguments and dispatches to the appropriate subcommand.
func run(args []string) error {
	// Parse global flags first.
	remaining, err := parseGlobalFlags(args)
	if err != nil {
		return err
	}

	if len(remaining) == 0 {
		printUsage()
		return fmt.Errorf("no command specified")
	}

	cmd := remaining[0]
	cmdArgs := remaining[1:]

	switch cmd {
	case "install-adapter":
		return cmdInstallAdapter(cmdArgs)
	case "list-adapters":
		return cmdListAdapters(cmdArgs)
	case "remove-adapter":
		return cmdRemoveAdapter(cmdArgs)
	case "adapter-status":
		return cmdAdapterStatus(cmdArgs)
	case "watch-adapters":
		return cmdWatchAdapters(cmdArgs)
	case "start-session":
		return cmdStartSession(cmdArgs)
	case "stop-session":
		return cmdStopSession(cmdArgs)
	case "get-status":
		return cmdGetStatus(cmdArgs)
	case "get-rate":
		return cmdGetRate(cmdArgs)
	case "lookup-zones":
		return cmdLookupZones(cmdArgs)
	case "zone-info":
		return cmdZoneInfo(cmdArgs)
	case "adapter-info":
		return cmdAdapterInfo(cmdArgs)
	case "help", "--help", "-h":
		printUsage()
		return nil
	default:
		printUsage()
		return fmt.Errorf("unknown command: %s", cmd)
	}
}

// parseGlobalFlags extracts global flags and returns remaining arguments.
func parseGlobalFlags(args []string) ([]string, error) {
	updateServiceAddr = envOrDefault("UPDATE_SERVICE_ADDR", "localhost:50053")
	adapterAddr = envOrDefault("ADAPTER_ADDR", "localhost:50054")
	parkingFeeServiceAddr = envOrDefault("PARKING_FEE_SERVICE_ADDR", "http://localhost:8080")

	var remaining []string
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "--update-service-addr":
			if i+1 >= len(args) {
				return nil, fmt.Errorf("--update-service-addr requires a value")
			}
			i++
			updateServiceAddr = args[i]
		case "--adapter-addr":
			if i+1 >= len(args) {
				return nil, fmt.Errorf("--adapter-addr requires a value")
			}
			i++
			adapterAddr = args[i]
		case "--parking-fee-service-addr":
			if i+1 >= len(args) {
				return nil, fmt.Errorf("--parking-fee-service-addr requires a value")
			}
			i++
			parkingFeeServiceAddr = args[i]
		default:
			remaining = append(remaining, args[i])
		}
	}
	return remaining, nil
}

func printUsage() {
	fmt.Fprintf(os.Stderr, `Usage: parking-app-cli [flags] <command>

UPDATE_SERVICE commands:
  install-adapter   --image-ref <ref> [--checksum <sha>]
  list-adapters
  remove-adapter    --adapter-id <id>
  adapter-status    --adapter-id <id>
  watch-adapters

PARKING_OPERATOR_ADAPTOR commands:
  start-session     --zone-id <zone> --vehicle-vin <vin>
  stop-session      --session-id <id>
  get-status        [--session-id <id>]
  get-rate          --zone-id <zone>

PARKING_FEE_SERVICE commands:
  lookup-zones      --lat <lat> --lon <lon>
  zone-info         --zone-id <id>
  adapter-info      --zone-id <id>

Global Flags:
  --update-service-addr        Address of UpdateService (default: localhost:50053)
  --adapter-addr               Address of ParkingAdapter (default: localhost:50054)
  --parking-fee-service-addr   Address of ParkingFeeService (default: http://localhost:8080)
`)
}

// dialGRPC creates a gRPC client connection to the given address.
func dialGRPC(addr string) (*grpc.ClientConn, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	conn, err := grpc.DialContext(ctx, addr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to %s: %w", addr, err)
	}
	return conn, nil
}

// printJSON marshals v as indented JSON and prints it to stdout.
func printJSON(v any) {
	data, err := json.MarshalIndent(v, "", "  ")
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to marshal response: %v\n", err)
		return
	}
	fmt.Println(string(data))
}

// ─── UpdateService subcommands ──────────────────────────────────────────────

func cmdInstallAdapter(args []string) error {
	imageRef := flagValue(args, "--image-ref", "")
	if imageRef == "" {
		return fmt.Errorf("--image-ref is required")
	}
	checksum := flagValue(args, "--checksum", "")

	conn, err := dialGRPC(updateServiceAddr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.InstallAdapter(ctx, &updatepb.InstallAdapterRequest{
		ImageRef: imageRef,
		Checksum: checksum,
	})
	if err != nil {
		return fmt.Errorf("InstallAdapter RPC failed: %w", err)
	}
	printJSON(resp)
	return nil
}

func cmdListAdapters(_ []string) error {
	conn, err := dialGRPC(updateServiceAddr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.ListAdapters(ctx, &updatepb.ListAdaptersRequest{})
	if err != nil {
		return fmt.Errorf("ListAdapters RPC failed: %w", err)
	}
	printJSON(resp)
	return nil
}

func cmdRemoveAdapter(args []string) error {
	adapterID := flagValue(args, "--adapter-id", "")
	if adapterID == "" {
		return fmt.Errorf("--adapter-id is required")
	}

	conn, err := dialGRPC(updateServiceAddr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.RemoveAdapter(ctx, &updatepb.RemoveAdapterRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		return fmt.Errorf("RemoveAdapter RPC failed: %w", err)
	}
	printJSON(resp)
	return nil
}

func cmdAdapterStatus(args []string) error {
	adapterID := flagValue(args, "--adapter-id", "")
	if adapterID == "" {
		return fmt.Errorf("--adapter-id is required")
	}

	conn, err := dialGRPC(updateServiceAddr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.GetAdapterStatus(ctx, &updatepb.GetAdapterStatusRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		return fmt.Errorf("GetAdapterStatus RPC failed: %w", err)
	}
	printJSON(resp)
	return nil
}

func cmdWatchAdapters(_ []string) error {
	conn, err := dialGRPC(updateServiceAddr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Handle SIGINT/SIGTERM to cleanly stop streaming.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigCh
		cancel()
	}()

	stream, err := client.WatchAdapterStates(ctx, &updatepb.WatchAdapterStatesRequest{})
	if err != nil {
		return fmt.Errorf("WatchAdapterStates RPC failed: %w", err)
	}

	fmt.Fprintln(os.Stderr, "Watching adapter state events (press Ctrl+C to stop)...")
	for {
		event, err := stream.Recv()
		if err == io.EOF {
			return nil
		}
		if err != nil {
			// Context cancellation from signal is a clean exit.
			if ctx.Err() != nil {
				fmt.Fprintln(os.Stderr, "\nStopped watching.")
				return nil
			}
			return fmt.Errorf("WatchAdapterStates stream error: %w", err)
		}
		printJSON(event)
	}
}

// ─── ParkingAdapter subcommands ─────────────────────────────────────────────

func cmdStartSession(args []string) error {
	zoneID := flagValue(args, "--zone-id", "")
	if zoneID == "" {
		return fmt.Errorf("--zone-id is required")
	}
	vin := flagValue(args, "--vehicle-vin", "")
	if vin == "" {
		return fmt.Errorf("--vehicle-vin is required")
	}

	conn, err := dialGRPC(adapterAddr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := adapterpb.NewParkingAdapterClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.StartSession(ctx, &adapterpb.StartSessionRequest{
		VehicleId: &commonpb.VehicleId{Vin: vin},
		ZoneId:    zoneID,
		Timestamp: time.Now().Unix(),
	})
	if err != nil {
		return fmt.Errorf("StartSession RPC failed: %w", err)
	}
	printJSON(resp)
	return nil
}

func cmdStopSession(args []string) error {
	sessionID := flagValue(args, "--session-id", "")
	if sessionID == "" {
		return fmt.Errorf("--session-id is required")
	}

	conn, err := dialGRPC(adapterAddr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := adapterpb.NewParkingAdapterClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.StopSession(ctx, &adapterpb.StopSessionRequest{
		SessionId: sessionID,
		Timestamp: time.Now().Unix(),
	})
	if err != nil {
		return fmt.Errorf("StopSession RPC failed: %w", err)
	}
	printJSON(resp)
	return nil
}

func cmdGetStatus(args []string) error {
	sessionID := flagValue(args, "--session-id", "")

	conn, err := dialGRPC(adapterAddr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := adapterpb.NewParkingAdapterClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.GetStatus(ctx, &adapterpb.GetStatusRequest{
		SessionId: sessionID,
	})
	if err != nil {
		return fmt.Errorf("GetStatus RPC failed: %w", err)
	}
	printJSON(resp)
	return nil
}

func cmdGetRate(args []string) error {
	zoneID := flagValue(args, "--zone-id", "")
	if zoneID == "" {
		return fmt.Errorf("--zone-id is required")
	}

	conn, err := dialGRPC(adapterAddr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := adapterpb.NewParkingAdapterClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.GetRate(ctx, &adapterpb.GetRateRequest{
		ZoneId: zoneID,
	})
	if err != nil {
		return fmt.Errorf("GetRate RPC failed: %w", err)
	}
	printJSON(resp)
	return nil
}

// ─── PARKING_FEE_SERVICE subcommands ─────────────────────────────────────────

// pfsErrorResponse represents a JSON error response from PARKING_FEE_SERVICE.
type pfsErrorResponse struct {
	Error string `json:"error"`
	Code  string `json:"code"`
}

// httpClient is the HTTP client used for PARKING_FEE_SERVICE requests.
// It is a package-level variable to allow tests to inject custom transports.
var httpClient = &http.Client{Timeout: 10 * time.Second}

// pfsGet performs a GET request against the PARKING_FEE_SERVICE and returns the
// response body. It returns an error if the service is unreachable or returns
// a non-2xx status code.
func pfsGet(path string) ([]byte, error) {
	url := strings.TrimRight(parkingFeeServiceAddr, "/") + path

	resp, err := httpClient.Get(url)
	if err != nil {
		return nil, fmt.Errorf("PARKING_FEE_SERVICE unreachable at %s: %w", parkingFeeServiceAddr, err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read response body: %w", err)
	}

	if resp.StatusCode >= 400 {
		var errResp pfsErrorResponse
		if json.Unmarshal(body, &errResp) == nil && errResp.Error != "" {
			return nil, fmt.Errorf("PARKING_FEE_SERVICE error (%d): %s", resp.StatusCode, errResp.Error)
		}
		return nil, fmt.Errorf("PARKING_FEE_SERVICE returned HTTP %d", resp.StatusCode)
	}

	return body, nil
}

func cmdLookupZones(args []string) error {
	latStr := flagValue(args, "--lat", "")
	if latStr == "" {
		return fmt.Errorf("--lat is required")
	}
	lonStr := flagValue(args, "--lon", "")
	if lonStr == "" {
		return fmt.Errorf("--lon is required")
	}

	// Validate that lat and lon are numeric.
	if _, err := strconv.ParseFloat(latStr, 64); err != nil {
		return fmt.Errorf("--lat must be a valid number: %s", latStr)
	}
	if _, err := strconv.ParseFloat(lonStr, 64); err != nil {
		return fmt.Errorf("--lon must be a valid number: %s", lonStr)
	}

	path := fmt.Sprintf("/api/v1/zones?lat=%s&lon=%s", latStr, lonStr)
	body, err := pfsGet(path)
	if err != nil {
		return err
	}

	// Parse and re-print as indented JSON.
	var zones []json.RawMessage
	if err := json.Unmarshal(body, &zones); err != nil {
		return fmt.Errorf("failed to parse zone lookup response: %w", err)
	}

	printJSON(zones)
	return nil
}

func cmdZoneInfo(args []string) error {
	zoneID := flagValue(args, "--zone-id", "")
	if zoneID == "" {
		return fmt.Errorf("--zone-id is required")
	}

	path := fmt.Sprintf("/api/v1/zones/%s", zoneID)
	body, err := pfsGet(path)
	if err != nil {
		return err
	}

	// Parse and re-print as indented JSON.
	var zone json.RawMessage
	if err := json.Unmarshal(body, &zone); err != nil {
		return fmt.Errorf("failed to parse zone info response: %w", err)
	}

	printJSON(zone)
	return nil
}

func cmdAdapterInfo(args []string) error {
	zoneID := flagValue(args, "--zone-id", "")
	if zoneID == "" {
		return fmt.Errorf("--zone-id is required")
	}

	path := fmt.Sprintf("/api/v1/zones/%s/adapter", zoneID)
	body, err := pfsGet(path)
	if err != nil {
		return err
	}

	// Parse and re-print as indented JSON.
	var adapter json.RawMessage
	if err := json.Unmarshal(body, &adapter); err != nil {
		return fmt.Errorf("failed to parse adapter info response: %w", err)
	}

	printJSON(adapter)
	return nil
}

// ─── Utility ────────────────────────────────────────────────────────────────

// flagValue extracts a flag value from args by name. Returns defaultVal if
// the flag is not present.
func flagValue(args []string, name, defaultVal string) string {
	for i, arg := range args {
		if arg == name && i+1 < len(args) {
			return args[i+1]
		}
	}
	return defaultVal
}

// envOrDefault returns the value of the given environment variable, or the
// default value if the variable is not set.
func envOrDefault(key, defaultVal string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return defaultVal
}
