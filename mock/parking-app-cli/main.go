// Package main implements parking-app-cli, a mock for the PARKING_APP on AAOS IVI.
// It queries PARKING_FEE_SERVICE (REST), manages adapter lifecycle via UPDATE_SERVICE (gRPC),
// and overrides adapter sessions via PARKING_OPERATOR_ADAPTOR (gRPC).
package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	adapterpb "github.com/rhadp/parking-fee-service/mock/parking-app-cli/pb/adapter"
	updatepb "github.com/rhadp/parking-fee-service/mock/parking-app-cli/pb/update"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// resolveAddr returns flag value if non-empty, else env var, else defaultVal.
func resolveAddr(flagVal, envVar, defaultVal string) string {
	if flagVal != "" {
		return flagVal
	}
	if v := os.Getenv(envVar); v != "" {
		return v
	}
	return defaultVal
}

// httpGet sends GET to url, prints JSON body to stdout, and returns an error on non-2xx.
func httpGet(url string) error {
	resp, err := http.Get(url) //nolint:noctx
	if err != nil {
		return fmt.Errorf("HTTP GET %s: %w", url, err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("read response: %w", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("HTTP %d: %s", resp.StatusCode, strings.TrimSpace(string(body)))
	}

	fmt.Println(string(body))
	return nil
}

// dialUpdate connects to UPDATE_SERVICE at addr and returns a client.
func dialUpdate(addr string) (updatepb.UpdateServiceClient, *grpc.ClientConn, error) {
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return nil, nil, fmt.Errorf("connect to UPDATE_SERVICE at %s: %w", addr, err)
	}
	return updatepb.NewUpdateServiceClient(conn), conn, nil
}

// dialAdapter connects to PARKING_OPERATOR_ADAPTOR at addr and returns a client.
func dialAdapter(addr string) (adapterpb.AdapterServiceClient, *grpc.ClientConn, error) {
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return nil, nil, fmt.Errorf("connect to PARKING_OPERATOR_ADAPTOR at %s: %w", addr, err)
	}
	return adapterpb.NewAdapterServiceClient(conn), conn, nil
}

// printJSON marshals v to indented JSON and writes it to stdout.
func printJSON(v any) {
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	enc.Encode(v) //nolint
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: parking-app-cli <command> [flags]")
		fmt.Fprintln(os.Stderr, "commands: lookup, adapter-info, install, list, status, remove, watch, start-session, stop-session")
		os.Exit(1)
	}

	subcommand := os.Args[1]

	switch subcommand {
	case "lookup":
		runLookup()
	case "adapter-info":
		runAdapterInfo()
	case "install":
		runInstall()
	case "list":
		runList()
	case "status":
		runAdapterStatus()
	case "remove":
		runRemove()
	case "watch":
		runWatch()
	case "start-session":
		runStartSession()
	case "stop-session":
		runStopSession()
	default:
		fmt.Fprintf(os.Stderr, "unknown command: %s\n", subcommand)
		fmt.Fprintln(os.Stderr, "commands: lookup, adapter-info, install, list, status, remove, watch, start-session, stop-session")
		os.Exit(1)
	}
}

func runLookup() {
	fs := flag.NewFlagSet("lookup", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	lat := fs.String("lat", "", "Latitude (required)")
	lon := fs.String("lon", "", "Longitude (required)")
	serviceAddr := fs.String("service-addr", "", "PARKING_FEE_SERVICE address")

	if err := fs.Parse(os.Args[2:]); err != nil {
		os.Exit(1)
	}

	if *lat == "" {
		fmt.Fprintln(os.Stderr, "error: --lat is required")
		fs.Usage()
		os.Exit(1)
	}
	if *lon == "" {
		fmt.Fprintln(os.Stderr, "error: --lon is required")
		fs.Usage()
		os.Exit(1)
	}

	addr := resolveAddr(*serviceAddr, "PARKING_FEE_SERVICE_ADDR", "http://localhost:8080")
	url := fmt.Sprintf("%s/operators?lat=%s&lon=%s", addr, *lat, *lon)
	if err := httpGet(url); err != nil {
		fmt.Fprintln(os.Stderr, "error:", err)
		os.Exit(1)
	}
}

func runAdapterInfo() {
	fs := flag.NewFlagSet("adapter-info", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	operatorID := fs.String("operator-id", "", "Operator ID (required)")
	serviceAddr := fs.String("service-addr", "", "PARKING_FEE_SERVICE address")

	if err := fs.Parse(os.Args[2:]); err != nil {
		os.Exit(1)
	}

	if *operatorID == "" {
		fmt.Fprintln(os.Stderr, "error: --operator-id is required")
		fs.Usage()
		os.Exit(1)
	}

	addr := resolveAddr(*serviceAddr, "PARKING_FEE_SERVICE_ADDR", "http://localhost:8080")
	url := fmt.Sprintf("%s/operators/%s/adapter", addr, *operatorID)
	if err := httpGet(url); err != nil {
		fmt.Fprintln(os.Stderr, "error:", err)
		os.Exit(1)
	}
}

func runInstall() {
	fs := flag.NewFlagSet("install", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	imageRef := fs.String("image-ref", "", "OCI image reference (required)")
	checksum := fs.String("checksum", "", "SHA-256 checksum (required)")
	updateAddr := fs.String("update-addr", "", "UPDATE_SERVICE address")

	if err := fs.Parse(os.Args[2:]); err != nil {
		os.Exit(1)
	}

	if *imageRef == "" {
		fmt.Fprintln(os.Stderr, "error: --image-ref is required")
		fs.Usage()
		os.Exit(1)
	}
	if *checksum == "" {
		fmt.Fprintln(os.Stderr, "error: --checksum is required")
		fs.Usage()
		os.Exit(1)
	}

	addr := resolveAddr(*updateAddr, "UPDATE_SERVICE_ADDR", "localhost:50052")
	client, conn, err := dialUpdate(addr)
	if err != nil {
		fmt.Fprintln(os.Stderr, "error:", err)
		os.Exit(1)
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.InstallAdapter(ctx, &updatepb.InstallAdapterRequest{
		ImageRef:       *imageRef,
		ChecksumSha256: *checksum,
	})
	if err != nil {
		fmt.Fprintln(os.Stderr, "gRPC error:", err)
		os.Exit(1)
	}

	printJSON(map[string]any{
		"job_id":     resp.JobId,
		"adapter_id": resp.AdapterId,
		"state":      resp.State.String(),
	})
}

func runList() {
	fs := flag.NewFlagSet("list", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	updateAddr := fs.String("update-addr", "", "UPDATE_SERVICE address")

	if err := fs.Parse(os.Args[2:]); err != nil {
		os.Exit(1)
	}

	addr := resolveAddr(*updateAddr, "UPDATE_SERVICE_ADDR", "localhost:50052")
	client, conn, err := dialUpdate(addr)
	if err != nil {
		fmt.Fprintln(os.Stderr, "error:", err)
		os.Exit(1)
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.ListAdapters(ctx, &updatepb.ListAdaptersRequest{})
	if err != nil {
		fmt.Fprintln(os.Stderr, "gRPC error:", err)
		os.Exit(1)
	}

	adapters := make([]map[string]any, 0, len(resp.Adapters))
	for _, a := range resp.Adapters {
		adapters = append(adapters, map[string]any{
			"adapter_id": a.AdapterId,
			"image_ref":  a.ImageRef,
			"state":      a.State.String(),
		})
	}
	printJSON(map[string]any{"adapters": adapters})
}

func runAdapterStatus() {
	fs := flag.NewFlagSet("status", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	adapterID := fs.String("adapter-id", "", "Adapter ID (required)")
	updateAddr := fs.String("update-addr", "", "UPDATE_SERVICE address")

	if err := fs.Parse(os.Args[2:]); err != nil {
		os.Exit(1)
	}

	if *adapterID == "" {
		fmt.Fprintln(os.Stderr, "error: --adapter-id is required")
		fs.Usage()
		os.Exit(1)
	}

	addr := resolveAddr(*updateAddr, "UPDATE_SERVICE_ADDR", "localhost:50052")
	client, conn, err := dialUpdate(addr)
	if err != nil {
		fmt.Fprintln(os.Stderr, "error:", err)
		os.Exit(1)
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.GetAdapterStatus(ctx, &updatepb.GetAdapterStatusRequest{
		AdapterId: *adapterID,
	})
	if err != nil {
		fmt.Fprintln(os.Stderr, "gRPC error:", err)
		os.Exit(1)
	}

	if resp.Adapter == nil {
		printJSON(map[string]any{"adapter_id": *adapterID, "state": "UNKNOWN"})
		return
	}
	printJSON(map[string]any{
		"adapter_id": resp.Adapter.AdapterId,
		"image_ref":  resp.Adapter.ImageRef,
		"state":      resp.Adapter.State.String(),
	})
}

func runRemove() {
	fs := flag.NewFlagSet("remove", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	adapterID := fs.String("adapter-id", "", "Adapter ID (required)")
	updateAddr := fs.String("update-addr", "", "UPDATE_SERVICE address")

	if err := fs.Parse(os.Args[2:]); err != nil {
		os.Exit(1)
	}

	if *adapterID == "" {
		fmt.Fprintln(os.Stderr, "error: --adapter-id is required")
		fs.Usage()
		os.Exit(1)
	}

	addr := resolveAddr(*updateAddr, "UPDATE_SERVICE_ADDR", "localhost:50052")
	client, conn, err := dialUpdate(addr)
	if err != nil {
		fmt.Fprintln(os.Stderr, "error:", err)
		os.Exit(1)
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.RemoveAdapter(ctx, &updatepb.RemoveAdapterRequest{
		AdapterId: *adapterID,
	})
	if err != nil {
		fmt.Fprintln(os.Stderr, "gRPC error:", err)
		os.Exit(1)
	}

	printJSON(map[string]any{
		"adapter_id": *adapterID,
		"removed":    resp.Success,
		"message":    resp.Message,
	})
}

func runWatch() {
	fs := flag.NewFlagSet("watch", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	updateAddr := fs.String("update-addr", "", "UPDATE_SERVICE address")

	if err := fs.Parse(os.Args[2:]); err != nil {
		os.Exit(1)
	}

	addr := resolveAddr(*updateAddr, "UPDATE_SERVICE_ADDR", "localhost:50052")
	client, conn, err := dialUpdate(addr)
	if err != nil {
		fmt.Fprintln(os.Stderr, "error:", err)
		os.Exit(1)
	}
	defer conn.Close()

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Cancel on SIGINT.
	sigs := make(chan os.Signal, 1)
	signal.Notify(sigs, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigs
		cancel()
	}()

	stream, err := client.WatchAdapterStates(ctx, &updatepb.WatchAdapterStatesRequest{})
	if err != nil {
		fmt.Fprintln(os.Stderr, "gRPC error:", err)
		os.Exit(1)
	}

	for {
		event, err := stream.Recv()
		if err != nil {
			if ctx.Err() != nil {
				// SIGINT received, clean exit
				return
			}
			fmt.Fprintln(os.Stderr, "stream closed:", err)
			return
		}
		printJSON(map[string]any{
			"adapter_id": event.AdapterId,
			"old_state":  event.OldState.String(),
			"new_state":  event.NewState.String(),
			"timestamp":  event.Timestamp,
		})
	}
}

func runStartSession() {
	fs := flag.NewFlagSet("start-session", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	zoneID := fs.String("zone-id", "", "Zone ID (required)")
	adaptorAddr := fs.String("adaptor-addr", "", "PARKING_OPERATOR_ADAPTOR address")

	if err := fs.Parse(os.Args[2:]); err != nil {
		os.Exit(1)
	}

	if *zoneID == "" {
		fmt.Fprintln(os.Stderr, "error: --zone-id is required")
		fs.Usage()
		os.Exit(1)
	}

	addr := resolveAddr(*adaptorAddr, "ADAPTOR_ADDR", "localhost:50053")
	client, conn, err := dialAdapter(addr)
	if err != nil {
		fmt.Fprintln(os.Stderr, "error:", err)
		os.Exit(1)
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.StartSession(ctx, &adapterpb.StartSessionRequest{
		ZoneId: *zoneID,
	})
	if err != nil {
		fmt.Fprintln(os.Stderr, "gRPC error:", err)
		os.Exit(1)
	}

	printJSON(map[string]any{
		"session_id": resp.SessionId,
	})
}

func runStopSession() {
	fs := flag.NewFlagSet("stop-session", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	adaptorAddr := fs.String("adaptor-addr", "", "PARKING_OPERATOR_ADAPTOR address")

	if err := fs.Parse(os.Args[2:]); err != nil {
		os.Exit(1)
	}

	addr := resolveAddr(*adaptorAddr, "ADAPTOR_ADDR", "localhost:50053")
	client, conn, err := dialAdapter(addr)
	if err != nil {
		fmt.Fprintln(os.Stderr, "error:", err)
		os.Exit(1)
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.StopSession(ctx, &adapterpb.StopSessionRequest{})
	if err != nil {
		fmt.Fprintln(os.Stderr, "gRPC error:", err)
		os.Exit(1)
	}

	printJSON(map[string]any{
		"stopped": resp.Success,
		"message": resp.Message,
	})
}
