package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	adaptorpb "github.com/rhadp/parking-fee-service/gen/parking_adaptor/v1"
	updatepb "github.com/rhadp/parking-fee-service/gen/update_service/v1"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "Usage: parking-app-cli <subcommand> [flags]")
		os.Exit(1)
	}

	subcmd := os.Args[1]

	switch subcmd {
	// REST subcommands (PARKING_FEE_SERVICE)
	case "lookup":
		runLookup()
	case "adapter-info":
		runAdapterInfo()

	// gRPC subcommands (UPDATE_SERVICE)
	case "install":
		runInstall()
	case "list":
		runList()
	case "watch":
		runWatch()
	case "status":
		runAdapterStatus()
	case "remove":
		runRemove()

	// gRPC subcommands (PARKING_OPERATOR_ADAPTOR)
	case "start-session":
		runStartSession()
	case "stop-session":
		runStopSession()

	default:
		fmt.Fprintf(os.Stderr, "unknown subcommand: %s\n", subcmd)
		os.Exit(1)
	}
}

// ---------------------------------------------------------------------------
// REST subcommands: lookup, adapter-info
// ---------------------------------------------------------------------------

func runLookup() {
	lat := getFlag("--lat")
	lon := getFlag("--lon")
	if lat == "" || lon == "" {
		fmt.Fprintln(os.Stderr, "error: --lat and --lon are required")
		os.Exit(1)
	}

	addr := resolveServiceAddr()
	url := fmt.Sprintf("%s/operators?lat=%s&lon=%s", addr, lat, lon)

	resp, err := http.Get(url)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
	defer resp.Body.Close()

	body, _ := io.ReadAll(resp.Body)

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		fmt.Fprintf(os.Stderr, "error: HTTP %d: %s\n", resp.StatusCode, string(body))
		os.Exit(1)
	}

	fmt.Print(string(body))
}

func runAdapterInfo() {
	operatorID := getFlag("--operator-id")
	if operatorID == "" {
		fmt.Fprintln(os.Stderr, "error: --operator-id is required")
		os.Exit(1)
	}

	addr := resolveServiceAddr()
	url := fmt.Sprintf("%s/operators/%s/adapter", addr, operatorID)

	resp, err := http.Get(url)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
	defer resp.Body.Close()

	body, _ := io.ReadAll(resp.Body)

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		fmt.Fprintf(os.Stderr, "error: HTTP %d: %s\n", resp.StatusCode, string(body))
		os.Exit(1)
	}

	fmt.Print(string(body))
}

// ---------------------------------------------------------------------------
// gRPC subcommands: install, list, watch, status, remove (UPDATE_SERVICE)
// ---------------------------------------------------------------------------

func runInstall() {
	imageRef := getFlag("--image-ref")
	checksum := getFlag("--checksum")
	if imageRef == "" || checksum == "" {
		fmt.Fprintln(os.Stderr, "error: --image-ref and --checksum are required")
		os.Exit(1)
	}

	addr := resolveUpdateAddr()
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		fmt.Fprintf(os.Stderr, "error connecting to UPDATE_SERVICE: %v\n", err)
		os.Exit(1)
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.InstallAdapter(ctx, &updatepb.InstallAdapterRequest{
		ImageRef:       imageRef,
		ChecksumSha256: checksum,
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: gRPC InstallAdapter failed: %v\n", err)
		os.Exit(1)
	}

	printJSON(map[string]interface{}{
		"job_id":     resp.GetJobId(),
		"adapter_id": resp.GetAdapterId(),
		"state":      resp.GetState().String(),
	})
}

func runList() {
	addr := resolveUpdateAddr()
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		fmt.Fprintf(os.Stderr, "error connecting to UPDATE_SERVICE: %v\n", err)
		os.Exit(1)
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.ListAdapters(ctx, &updatepb.ListAdaptersRequest{})
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: gRPC ListAdapters failed: %v\n", err)
		os.Exit(1)
	}

	adapters := make([]map[string]interface{}, 0, len(resp.GetAdapters()))
	for _, a := range resp.GetAdapters() {
		adapters = append(adapters, map[string]interface{}{
			"adapter_id": a.GetAdapterId(),
			"state":      a.GetState().String(),
			"image_ref":  a.GetImageRef(),
		})
	}

	printJSON(map[string]interface{}{
		"adapters": adapters,
	})
}

func runWatch() {
	addr := resolveUpdateAddr()
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		fmt.Fprintf(os.Stderr, "error connecting to UPDATE_SERVICE: %v\n", err)
		os.Exit(1)
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Handle SIGINT for graceful shutdown.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT)
	go func() {
		<-sigCh
		cancel()
	}()

	stream, err := client.WatchAdapterStates(ctx, &updatepb.WatchAdapterStatesRequest{})
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: gRPC WatchAdapterStates failed: %v\n", err)
		os.Exit(1)
	}

	for {
		event, err := stream.Recv()
		if err != nil {
			// Stream closed or context cancelled — normal exit.
			break
		}
		printJSON(map[string]interface{}{
			"adapter_id": event.GetAdapterId(),
			"old_state":  event.GetOldState().String(),
			"new_state":  event.GetNewState().String(),
			"timestamp":  event.GetTimestamp(),
		})
	}
}

func runAdapterStatus() {
	adapterID := getFlag("--adapter-id")
	if adapterID == "" {
		fmt.Fprintln(os.Stderr, "error: --adapter-id is required")
		os.Exit(1)
	}

	addr := resolveUpdateAddr()
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		fmt.Fprintf(os.Stderr, "error connecting to UPDATE_SERVICE: %v\n", err)
		os.Exit(1)
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.GetAdapterStatus(ctx, &updatepb.GetAdapterStatusRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: gRPC GetAdapterStatus failed: %v\n", err)
		os.Exit(1)
	}

	printJSON(map[string]interface{}{
		"adapter_id": resp.GetAdapterId(),
		"state":      resp.GetState().String(),
		"image_ref":  resp.GetImageRef(),
	})
}

func runRemove() {
	adapterID := getFlag("--adapter-id")
	if adapterID == "" {
		fmt.Fprintln(os.Stderr, "error: --adapter-id is required")
		os.Exit(1)
	}

	addr := resolveUpdateAddr()
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		fmt.Fprintf(os.Stderr, "error connecting to UPDATE_SERVICE: %v\n", err)
		os.Exit(1)
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	_, err = client.RemoveAdapter(ctx, &updatepb.RemoveAdapterRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: gRPC RemoveAdapter failed: %v\n", err)
		os.Exit(1)
	}

	printJSON(map[string]interface{}{
		"adapter_id": adapterID,
		"removed":    true,
	})
}

// ---------------------------------------------------------------------------
// gRPC subcommands: start-session, stop-session (PARKING_OPERATOR_ADAPTOR)
// ---------------------------------------------------------------------------

func runStartSession() {
	zoneID := getFlag("--zone-id")
	if zoneID == "" {
		fmt.Fprintln(os.Stderr, "error: --zone-id is required")
		os.Exit(1)
	}

	addr := resolveAdaptorAddr()
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		fmt.Fprintf(os.Stderr, "error connecting to PARKING_OPERATOR_ADAPTOR: %v\n", err)
		os.Exit(1)
	}
	defer conn.Close()

	client := adaptorpb.NewParkingAdaptorClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.StartSession(ctx, &adaptorpb.StartSessionRequest{
		ZoneId: zoneID,
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: gRPC StartSession failed: %v\n", err)
		os.Exit(1)
	}

	result := map[string]interface{}{
		"session_id": resp.GetSessionId(),
		"status":     resp.GetStatus(),
	}
	if resp.GetRate() != nil {
		result["rate"] = map[string]interface{}{
			"rate_type": resp.GetRate().GetRateType(),
			"amount":    resp.GetRate().GetAmount(),
			"currency":  resp.GetRate().GetCurrency(),
		}
	}
	printJSON(result)
}

func runStopSession() {
	addr := resolveAdaptorAddr()
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		fmt.Fprintf(os.Stderr, "error connecting to PARKING_OPERATOR_ADAPTOR: %v\n", err)
		os.Exit(1)
	}
	defer conn.Close()

	client := adaptorpb.NewParkingAdaptorClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.StopSession(ctx, &adaptorpb.StopSessionRequest{})
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: gRPC StopSession failed: %v\n", err)
		os.Exit(1)
	}

	printJSON(map[string]interface{}{
		"session_id":       resp.GetSessionId(),
		"status":           resp.GetStatus(),
		"duration_seconds": resp.GetDurationSeconds(),
		"total_amount":     resp.GetTotalAmount(),
		"currency":         resp.GetCurrency(),
	})
}

// ---------------------------------------------------------------------------
// Address resolution helpers
// ---------------------------------------------------------------------------

func resolveServiceAddr() string {
	if addr := getFlag("--service-addr"); addr != "" {
		return addr
	}
	if addr := os.Getenv("PARKING_FEE_SERVICE_ADDR"); addr != "" {
		return addr
	}
	return "http://localhost:8080"
}

func resolveUpdateAddr() string {
	if addr := getFlag("--update-addr"); addr != "" {
		return addr
	}
	if addr := os.Getenv("UPDATE_SERVICE_ADDR"); addr != "" {
		return addr
	}
	return "localhost:50052"
}

func resolveAdaptorAddr() string {
	if addr := getFlag("--adaptor-addr"); addr != "" {
		return addr
	}
	if addr := os.Getenv("ADAPTOR_ADDR"); addr != "" {
		return addr
	}
	return "localhost:50053"
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

// getFlag extracts a --key=value style flag from os.Args.
func getFlag(name string) string {
	prefix := name + "="
	for _, arg := range os.Args[2:] {
		if val, ok := strings.CutPrefix(arg, prefix); ok {
			return val
		}
	}
	return ""
}

// printJSON marshals v to JSON and prints it to stdout.
func printJSON(v interface{}) {
	data, err := json.MarshalIndent(v, "", "  ")
	if err != nil {
		fmt.Fprintf(os.Stderr, "error marshaling JSON: %v\n", err)
		os.Exit(1)
	}
	fmt.Println(string(data))
}
