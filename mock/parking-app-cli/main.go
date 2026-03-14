package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"
	"time"

	"github.com/rhadp/parking-fee-service/gen/go/parkingadaptorpb"
	"github.com/rhadp/parking-fee-service/gen/go/updateservicepb"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// config holds runtime configuration for parking-app-cli.
// Satisfies: 09-REQ-5.3
type config struct {
	FeeServiceURL   string // PARKING_FEE_SERVICE base URL
	UpdateSvcAddr   string // UPDATE_SERVICE gRPC address
	AdaptorAddr     string // PARKING_OPERATOR_ADAPTOR gRPC address
}

// loadConfig reads configuration from environment variables and flags.
// Flags take precedence over env vars, which take precedence over defaults.
func loadConfig(args []string) config {
	cfg := config{
		FeeServiceURL: "http://localhost:8080",
		UpdateSvcAddr: "localhost:50052",
		AdaptorAddr:   "localhost:50053",
	}
	if v := os.Getenv("PARKING_FEE_SERVICE_URL"); v != "" {
		cfg.FeeServiceURL = v
	}
	if v := os.Getenv("UPDATE_SERVICE_ADDR"); v != "" {
		cfg.UpdateSvcAddr = v
	}
	if v := os.Getenv("ADAPTOR_ADDR"); v != "" {
		cfg.AdaptorAddr = v
	}
	// Parse flags.
	for _, arg := range args {
		switch {
		case strings.HasPrefix(arg, "--fee-service-url="):
			cfg.FeeServiceURL = arg[len("--fee-service-url="):]
		case strings.HasPrefix(arg, "--update-service-addr="):
			cfg.UpdateSvcAddr = arg[len("--update-service-addr="):]
		case strings.HasPrefix(arg, "--adaptor-addr="):
			cfg.AdaptorAddr = arg[len("--adaptor-addr="):]
		}
	}
	return cfg
}

// run is the testable entry-point for parking-app-cli.
func run(args []string, stdout, stderr io.Writer) int {
	if len(args) == 0 || args[0] == "--help" || args[0] == "-h" {
		printUsage(stderr)
		if len(args) > 0 {
			return 0
		}
		return 1
	}

	subcommand := args[0]
	rest := args[1:]
	cfg := loadConfig(rest)

	// Parse common flags.
	var lat, lon, operatorID, imageRef, checksum, adapterID, zoneID, commandID string
	for _, arg := range rest {
		switch {
		case strings.HasPrefix(arg, "--lat="):
			lat = arg[len("--lat="):]
		case strings.HasPrefix(arg, "--lon="):
			lon = arg[len("--lon="):]
		case strings.HasPrefix(arg, "--operator-id="):
			operatorID = arg[len("--operator-id="):]
		case strings.HasPrefix(arg, "--image-ref="):
			imageRef = arg[len("--image-ref="):]
		case strings.HasPrefix(arg, "--checksum="):
			checksum = arg[len("--checksum="):]
		case strings.HasPrefix(arg, "--adapter-id="):
			adapterID = arg[len("--adapter-id="):]
		case strings.HasPrefix(arg, "--zone-id="):
			zoneID = arg[len("--zone-id="):]
		case strings.HasPrefix(arg, "--command-id="):
			commandID = arg[len("--command-id="):]
		}
	}
	_ = commandID // may be used in future subcommands

	client := &http.Client{}

	switch subcommand {
	case "lookup":
		if lat == "" || lon == "" {
			fmt.Fprintln(stderr, "error: --lat and --lon are required for lookup")
			return 1
		}
		return cmdLookup(client, cfg, lat, lon, stdout, stderr)

	case "adapter-info":
		if operatorID == "" {
			fmt.Fprintln(stderr, "error: --operator-id is required for adapter-info")
			return 1
		}
		return cmdAdapterInfo(client, cfg, operatorID, stdout, stderr)

	case "install":
		if imageRef == "" || checksum == "" {
			fmt.Fprintln(stderr, "error: --image-ref and --checksum are required for install")
			return 1
		}
		return cmdInstall(cfg, imageRef, checksum, stdout, stderr)

	case "watch":
		return cmdWatch(cfg, stdout, stderr)

	case "list":
		return cmdList(cfg, stdout, stderr)

	case "remove":
		if adapterID == "" {
			fmt.Fprintln(stderr, "error: --adapter-id is required for remove")
			return 1
		}
		return cmdRemove(cfg, adapterID, stdout, stderr)

	case "status":
		if adapterID == "" {
			fmt.Fprintln(stderr, "error: --adapter-id is required for status")
			return 1
		}
		return cmdAdapterStatus(cfg, adapterID, stdout, stderr)

	case "start-session":
		if zoneID == "" {
			fmt.Fprintln(stderr, "error: --zone-id is required for start-session")
			return 1
		}
		return cmdStartSession(cfg, zoneID, stdout, stderr)

	case "stop-session":
		return cmdStopSession(cfg, stdout, stderr)

	default:
		fmt.Fprintf(stderr, "unknown subcommand: %q\n", subcommand)
		printUsage(stderr)
		return 1
	}
}

// cmdLookup queries PARKING_FEE_SERVICE for nearby operators.
// Satisfies: 09-REQ-4.1
func cmdLookup(client *http.Client, cfg config, lat, lon string, stdout, stderr io.Writer) int {
	url := fmt.Sprintf("%s/operators?lat=%s&lon=%s", cfg.FeeServiceURL, lat, lon)
	resp, err := client.Get(url)
	if err != nil {
		fmt.Fprintf(stderr, "error: failed to connect to PARKING_FEE_SERVICE at %s: %v\n", cfg.FeeServiceURL, err)
		return 1
	}
	defer resp.Body.Close()
	return printHTTPResponse(resp, stdout, stderr)
}

// cmdAdapterInfo queries PARKING_FEE_SERVICE for adapter metadata.
// Satisfies: 09-REQ-4.2
func cmdAdapterInfo(client *http.Client, cfg config, operatorID string, stdout, stderr io.Writer) int {
	url := fmt.Sprintf("%s/operators/%s/adapter", cfg.FeeServiceURL, operatorID)
	resp, err := client.Get(url)
	if err != nil {
		fmt.Fprintf(stderr, "error: failed to connect to PARKING_FEE_SERVICE at %s: %v\n", cfg.FeeServiceURL, err)
		return 1
	}
	defer resp.Body.Close()
	return printHTTPResponse(resp, stdout, stderr)
}

// printHTTPResponse prints the body to stdout; on non-2xx prints to stderr and returns 1.
func printHTTPResponse(resp *http.Response, stdout, stderr io.Writer) int {
	body, _ := io.ReadAll(resp.Body)
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		fmt.Fprintf(stderr, "error: upstream returned HTTP %d: %s\n", resp.StatusCode, string(body))
		return 1
	}
	fmt.Fprintln(stdout, string(body))
	return 0
}

// newUpdateServiceClient dials UPDATE_SERVICE and returns a client.
func newUpdateServiceClient(addr string) (updateservicepb.UpdateServiceClient, *grpc.ClientConn, error) {
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return nil, nil, err
	}
	return updateservicepb.NewUpdateServiceClient(conn), conn, nil
}

// newAdaptorClient dials PARKING_OPERATOR_ADAPTOR and returns a client.
func newAdaptorClient(addr string) (parkingadaptorpb.ParkingAdaptorClient, *grpc.ClientConn, error) {
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return nil, nil, err
	}
	return parkingadaptorpb.NewParkingAdaptorClient(conn), conn, nil
}

// cmdInstall calls UPDATE_SERVICE InstallAdapter. Satisfies: 09-REQ-4.3
func cmdInstall(cfg config, imageRef, checksum string, stdout, stderr io.Writer) int {
	client, conn, err := newUpdateServiceClient(cfg.UpdateSvcAddr)
	if err != nil {
		fmt.Fprintf(stderr, "error: failed to connect to UPDATE_SERVICE at %s: %v\n", cfg.UpdateSvcAddr, err)
		return 1
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	resp, err := client.InstallAdapter(ctx, &updateservicepb.InstallAdapterRequest{
		ImageRef:       imageRef,
		ChecksumSha256: checksum,
	})
	if err != nil {
		fmt.Fprintf(stderr, "error: InstallAdapter failed: %v\n", err)
		return 1
	}
	b, _ := json.MarshalIndent(resp, "", "  ")
	fmt.Fprintln(stdout, string(b))
	return 0
}

// cmdWatch streams WatchAdapterStates events. Satisfies: 09-REQ-4.4
func cmdWatch(cfg config, stdout, stderr io.Writer) int {
	client, conn, err := newUpdateServiceClient(cfg.UpdateSvcAddr)
	if err != nil {
		fmt.Fprintf(stderr, "error: failed to connect to UPDATE_SERVICE at %s: %v\n", cfg.UpdateSvcAddr, err)
		return 1
	}
	defer conn.Close()

	ctx := context.Background()
	stream, err := client.WatchAdapterStates(ctx, &updateservicepb.WatchAdapterStatesRequest{})
	if err != nil {
		fmt.Fprintf(stderr, "error: WatchAdapterStates failed: %v\n", err)
		return 1
	}
	for {
		event, err := stream.Recv()
		if err != nil {
			break
		}
		b, _ := json.MarshalIndent(event, "", "  ")
		fmt.Fprintln(stdout, string(b))
	}
	return 0
}

// cmdList calls UPDATE_SERVICE ListAdapters. Satisfies: 09-REQ-4.5
func cmdList(cfg config, stdout, stderr io.Writer) int {
	client, conn, err := newUpdateServiceClient(cfg.UpdateSvcAddr)
	if err != nil {
		fmt.Fprintf(stderr, "error: failed to connect to UPDATE_SERVICE at %s: %v\n", cfg.UpdateSvcAddr, err)
		return 1
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	resp, err := client.ListAdapters(ctx, &updateservicepb.ListAdaptersRequest{})
	if err != nil {
		fmt.Fprintf(stderr, "error: ListAdapters failed: %v\n", err)
		return 1
	}
	b, _ := json.MarshalIndent(resp, "", "  ")
	fmt.Fprintln(stdout, string(b))
	return 0
}

// cmdRemove calls UPDATE_SERVICE RemoveAdapter. Satisfies: 09-REQ-4.6
func cmdRemove(cfg config, adapterID string, stdout, stderr io.Writer) int {
	client, conn, err := newUpdateServiceClient(cfg.UpdateSvcAddr)
	if err != nil {
		fmt.Fprintf(stderr, "error: failed to connect to UPDATE_SERVICE at %s: %v\n", cfg.UpdateSvcAddr, err)
		return 1
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	resp, err := client.RemoveAdapter(ctx, &updateservicepb.RemoveAdapterRequest{AdapterId: adapterID})
	if err != nil {
		fmt.Fprintf(stderr, "error: RemoveAdapter failed: %v\n", err)
		return 1
	}
	b, _ := json.MarshalIndent(resp, "", "  ")
	fmt.Fprintln(stdout, string(b))
	return 0
}

// cmdAdapterStatus calls UPDATE_SERVICE GetAdapterStatus. Satisfies: 09-REQ-4.7
func cmdAdapterStatus(cfg config, adapterID string, stdout, stderr io.Writer) int {
	client, conn, err := newUpdateServiceClient(cfg.UpdateSvcAddr)
	if err != nil {
		fmt.Fprintf(stderr, "error: failed to connect to UPDATE_SERVICE at %s: %v\n", cfg.UpdateSvcAddr, err)
		return 1
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	resp, err := client.GetAdapterStatus(ctx, &updateservicepb.GetAdapterStatusRequest{AdapterId: adapterID})
	if err != nil {
		fmt.Fprintf(stderr, "error: GetAdapterStatus failed: %v\n", err)
		return 1
	}
	b, _ := json.MarshalIndent(resp, "", "  ")
	fmt.Fprintln(stdout, string(b))
	return 0
}

// cmdStartSession calls PARKING_OPERATOR_ADAPTOR StartSession. Satisfies: 09-REQ-4.8
func cmdStartSession(cfg config, zoneID string, stdout, stderr io.Writer) int {
	client, conn, err := newAdaptorClient(cfg.AdaptorAddr)
	if err != nil {
		fmt.Fprintf(stderr, "error: failed to connect to PARKING_OPERATOR_ADAPTOR at %s: %v\n", cfg.AdaptorAddr, err)
		return 1
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	resp, err := client.StartSession(ctx, &parkingadaptorpb.StartSessionRequest{ZoneId: zoneID})
	if err != nil {
		fmt.Fprintf(stderr, "error: StartSession failed: %v\n", err)
		return 1
	}
	b, _ := json.MarshalIndent(resp, "", "  ")
	fmt.Fprintln(stdout, string(b))
	return 0
}

// cmdStopSession calls PARKING_OPERATOR_ADAPTOR StopSession. Satisfies: 09-REQ-4.9
func cmdStopSession(cfg config, stdout, stderr io.Writer) int {
	client, conn, err := newAdaptorClient(cfg.AdaptorAddr)
	if err != nil {
		fmt.Fprintf(stderr, "error: failed to connect to PARKING_OPERATOR_ADAPTOR at %s: %v\n", cfg.AdaptorAddr, err)
		return 1
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	resp, err := client.StopSession(ctx, &parkingadaptorpb.StopSessionRequest{})
	if err != nil {
		fmt.Fprintf(stderr, "error: StopSession failed: %v\n", err)
		return 1
	}
	b, _ := json.MarshalIndent(resp, "", "  ")
	fmt.Fprintln(stdout, string(b))
	return 0
}

func printUsage(w io.Writer) {
	fmt.Fprintln(w, "Usage: parking-app-cli <subcommand> [options]")
	fmt.Fprintln(w, "")
	fmt.Fprintln(w, "Subcommands:")
	fmt.Fprintln(w, "  lookup          --lat=<lat> --lon=<lon>")
	fmt.Fprintln(w, "  adapter-info    --operator-id=<id>")
	fmt.Fprintln(w, "  install         --image-ref=<ref> --checksum=<sha256>")
	fmt.Fprintln(w, "  watch")
	fmt.Fprintln(w, "  list")
	fmt.Fprintln(w, "  remove          --adapter-id=<id>")
	fmt.Fprintln(w, "  status          --adapter-id=<id>")
	fmt.Fprintln(w, "  start-session   --zone-id=<zone>")
	fmt.Fprintln(w, "  stop-session")
	fmt.Fprintln(w, "")
	fmt.Fprintln(w, "Environment variables:")
	fmt.Fprintln(w, "  PARKING_FEE_SERVICE_URL  (default http://localhost:8080)")
	fmt.Fprintln(w, "  UPDATE_SERVICE_ADDR      (default localhost:50052)")
	fmt.Fprintln(w, "  ADAPTOR_ADDR             (default localhost:50053)")
}

func main() {
	os.Exit(run(os.Args[1:], os.Stdout, os.Stderr))
}
