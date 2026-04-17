// parking-app-cli simulates the PARKING_APP on AAOS IVI. It queries
// PARKING_FEE_SERVICE (REST), manages adapters via UPDATE_SERVICE (gRPC),
// and overrides sessions via PARKING_OPERATOR_ADAPTOR (gRPC).
//
// Subcommands:
//
//	lookup         --lat=<lat> --lon=<lon>          [--service-addr=<addr>]
//	adapter-info   --operator-id=<id>               [--service-addr=<addr>]
//	install        --image-ref=<ref> --checksum=<sha256> [--update-addr=<addr>]
//	list                                            [--update-addr=<addr>]
//	status         --adapter-id=<id>               [--update-addr=<addr>]
//	remove         --adapter-id=<id>               [--update-addr=<addr>]
//	watch                                           [--update-addr=<addr>]
//	start-session  --zone-id=<zone>                [--adaptor-addr=<addr>]
//	stop-session                                    [--adaptor-addr=<addr>]
package main

import (
	"bytes"
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

	adapterpb "github.com/sdv-demo/mock/parking-app-cli/pb/adapter"
	updatepb "github.com/sdv-demo/mock/parking-app-cli/pb/update"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

const (
	defaultServiceAddr = "http://localhost:8080"
	defaultUpdateAddr  = "localhost:50052"
	defaultAdaptorAddr = "localhost:50053"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: parking-app-cli <subcommand> [flags]")
		fmt.Fprintln(os.Stderr, "subcommands: lookup, adapter-info, install, list, status, remove, watch, start-session, stop-session")
		os.Exit(1)
	}

	subcommand := os.Args[1]
	args := os.Args[2:]

	switch subcommand {
	case "lookup":
		runLookup(args)
	case "adapter-info":
		runAdapterInfo(args)
	case "install":
		runInstall(args)
	case "list":
		runList(args)
	case "status":
		runStatus(args)
	case "remove":
		runRemove(args)
	case "watch":
		runWatch(args)
	case "start-session":
		runStartSession(args)
	case "stop-session":
		runStopSession(args)
	default:
		fmt.Fprintf(os.Stderr, "unknown subcommand: %s\n", subcommand)
		fmt.Fprintln(os.Stderr, "subcommands: lookup, adapter-info, install, list, status, remove, watch, start-session, stop-session")
		os.Exit(1)
	}
}

// ── Address helpers ──────────────────────────────────────────────────────────

func resolveServiceAddr(flag string) string {
	if flag != "" {
		return flag
	}
	if v := os.Getenv("PARKING_FEE_SERVICE_ADDR"); v != "" {
		return v
	}
	return defaultServiceAddr
}

func resolveUpdateAddr(flag string) string {
	if flag != "" {
		return flag
	}
	if v := os.Getenv("UPDATE_SERVICE_ADDR"); v != "" {
		return v
	}
	return defaultUpdateAddr
}

func resolveAdaptorAddr(flag string) string {
	if flag != "" {
		return flag
	}
	if v := os.Getenv("ADAPTOR_ADDR"); v != "" {
		return v
	}
	return defaultAdaptorAddr
}

// ── REST helpers ─────────────────────────────────────────────────────────────

func restGet(url string) {
	resp, err := http.Get(url) //nolint:noctx
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error reading response: %v\n", err)
		os.Exit(1)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		fmt.Fprintf(os.Stderr, "HTTP %d: %s\n", resp.StatusCode, strings.TrimSpace(string(body)))
		os.Exit(1)
	}

	printJSON(body)
}

func printJSON(data []byte) {
	var pretty bytes.Buffer
	if json.Indent(&pretty, data, "", "  ") == nil {
		fmt.Println(pretty.String())
	} else {
		fmt.Println(string(data))
	}
}

func printAny(v any) {
	data, err := json.MarshalIndent(v, "", "  ")
	if err != nil {
		fmt.Println(v)
		return
	}
	fmt.Println(string(data))
}

// ── gRPC helpers ─────────────────────────────────────────────────────────────

func dialUpdate(addr string) (*grpc.ClientConn, updatepb.UpdateServiceClient) {
	conn, err := grpc.NewClient(addr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error connecting to UPDATE_SERVICE at %s: %v\n", addr, err)
		os.Exit(1)
	}
	return conn, updatepb.NewUpdateServiceClient(conn)
}

func dialAdaptor(addr string) (*grpc.ClientConn, adapterpb.AdapterServiceClient) {
	conn, err := grpc.NewClient(addr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error connecting to PARKING_OPERATOR_ADAPTOR at %s: %v\n", addr, err)
		os.Exit(1)
	}
	return conn, adapterpb.NewAdapterServiceClient(conn)
}

// ── REST subcommands ─────────────────────────────────────────────────────────

func runLookup(args []string) {
	fs := flag.NewFlagSet("lookup", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	// Use string flags to preserve the exact user-supplied representation
	// (e.g., --lon=11.5820 stays "11.5820", not "11.582").
	lat := fs.String("lat", "", "latitude (required)")
	lon := fs.String("lon", "", "longitude (required)")
	serviceAddr := fs.String("service-addr", "", "PARKING_FEE_SERVICE address")

	if err := fs.Parse(args); err != nil {
		os.Exit(1)
	}

	if *lat == "" || *lon == "" {
		fmt.Fprintln(os.Stderr, "error: --lat and --lon are required")
		fs.Usage()
		os.Exit(1)
	}

	base := resolveServiceAddr(*serviceAddr)
	url := fmt.Sprintf("%s/operators?lat=%s&lon=%s",
		strings.TrimRight(base, "/"), *lat, *lon)
	restGet(url)
}

func runAdapterInfo(args []string) {
	fs := flag.NewFlagSet("adapter-info", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	operatorID := fs.String("operator-id", "", "operator ID (required)")
	serviceAddr := fs.String("service-addr", "", "PARKING_FEE_SERVICE address")

	if err := fs.Parse(args); err != nil {
		os.Exit(1)
	}

	if *operatorID == "" {
		fmt.Fprintln(os.Stderr, "error: --operator-id is required")
		fs.Usage()
		os.Exit(1)
	}

	base := resolveServiceAddr(*serviceAddr)
	url := fmt.Sprintf("%s/operators/%s/adapter",
		strings.TrimRight(base, "/"), *operatorID)
	restGet(url)
}

// ── gRPC UPDATE_SERVICE subcommands ─────────────────────────────────────────

func runInstall(args []string) {
	fs := flag.NewFlagSet("install", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	imageRef := fs.String("image-ref", "", "container image reference (required)")
	checksum := fs.String("checksum", "", "SHA-256 checksum (required)")
	updateAddr := fs.String("update-addr", "", "UPDATE_SERVICE address")

	if err := fs.Parse(args); err != nil {
		os.Exit(1)
	}

	if *imageRef == "" || *checksum == "" {
		fmt.Fprintln(os.Stderr, "error: --image-ref and --checksum are required")
		fs.Usage()
		os.Exit(1)
	}

	addr := resolveUpdateAddr(*updateAddr)
	conn, client := dialUpdate(addr)
	defer conn.Close()

	resp, err := client.InstallAdapter(context.Background(), &updatepb.InstallAdapterRequest{
		ImageRef:       *imageRef,
		ChecksumSha256: *checksum,
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "gRPC error: %v\n", err)
		os.Exit(1)
	}

	printAny(map[string]any{
		"job_id":     resp.JobId,
		"adapter_id": resp.AdapterId,
		"state":      resp.State.String(),
	})
}

func runList(args []string) {
	fs := flag.NewFlagSet("list", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	updateAddr := fs.String("update-addr", "", "UPDATE_SERVICE address")

	if err := fs.Parse(args); err != nil {
		os.Exit(1)
	}

	addr := resolveUpdateAddr(*updateAddr)
	conn, client := dialUpdate(addr)
	defer conn.Close()

	resp, err := client.ListAdapters(context.Background(), &updatepb.ListAdaptersRequest{})
	if err != nil {
		fmt.Fprintf(os.Stderr, "gRPC error: %v\n", err)
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
	printAny(map[string]any{"adapters": adapters})
}

func runStatus(args []string) {
	fs := flag.NewFlagSet("status", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	adapterID := fs.String("adapter-id", "", "adapter ID (required)")
	updateAddr := fs.String("update-addr", "", "UPDATE_SERVICE address")

	if err := fs.Parse(args); err != nil {
		os.Exit(1)
	}

	if *adapterID == "" {
		fmt.Fprintln(os.Stderr, "error: --adapter-id is required")
		fs.Usage()
		os.Exit(1)
	}

	addr := resolveUpdateAddr(*updateAddr)
	conn, client := dialUpdate(addr)
	defer conn.Close()

	resp, err := client.GetAdapterStatus(context.Background(), &updatepb.GetAdapterStatusRequest{
		AdapterId: *adapterID,
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "gRPC error: %v\n", err)
		os.Exit(1)
	}

	printAny(map[string]any{
		"adapter_id": resp.AdapterId,
		"image_ref":  resp.ImageRef,
		"state":      resp.State.String(),
	})
}

func runRemove(args []string) {
	fs := flag.NewFlagSet("remove", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	adapterID := fs.String("adapter-id", "", "adapter ID (required)")
	updateAddr := fs.String("update-addr", "", "UPDATE_SERVICE address")

	if err := fs.Parse(args); err != nil {
		os.Exit(1)
	}

	if *adapterID == "" {
		fmt.Fprintln(os.Stderr, "error: --adapter-id is required")
		fs.Usage()
		os.Exit(1)
	}

	addr := resolveUpdateAddr(*updateAddr)
	conn, client := dialUpdate(addr)
	defer conn.Close()

	_, err := client.RemoveAdapter(context.Background(), &updatepb.RemoveAdapterRequest{
		AdapterId: *adapterID,
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "gRPC error: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("adapter %s removed\n", *adapterID)
}

func runWatch(args []string) {
	fs := flag.NewFlagSet("watch", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	updateAddr := fs.String("update-addr", "", "UPDATE_SERVICE address")

	if err := fs.Parse(args); err != nil {
		os.Exit(1)
	}

	addr := resolveUpdateAddr(*updateAddr)
	conn, client := dialUpdate(addr)
	defer conn.Close()

	ctx, cancel := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer cancel()

	stream, err := client.WatchAdapterStates(ctx, &updatepb.WatchAdapterStatesRequest{})
	if err != nil {
		fmt.Fprintf(os.Stderr, "gRPC error: %v\n", err)
		os.Exit(1)
	}

	for {
		event, err := stream.Recv()
		if err != nil {
			if ctx.Err() != nil {
				// SIGINT or SIGTERM — clean exit.
				return
			}
			fmt.Fprintf(os.Stderr, "stream error: %v\n", err)
			os.Exit(1)
		}
		printAny(map[string]any{
			"adapter_id": event.AdapterId,
			"old_state":  event.OldState.String(),
			"new_state":  event.NewState.String(),
			"timestamp":  event.Timestamp,
		})
	}
}

// ── gRPC PARKING_OPERATOR_ADAPTOR subcommands ────────────────────────────────

func runStartSession(args []string) {
	fs := flag.NewFlagSet("start-session", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	zoneID := fs.String("zone-id", "", "parking zone ID (required)")
	adaptorAddr := fs.String("adaptor-addr", "", "PARKING_OPERATOR_ADAPTOR address")

	if err := fs.Parse(args); err != nil {
		os.Exit(1)
	}

	if *zoneID == "" {
		fmt.Fprintln(os.Stderr, "error: --zone-id is required")
		fs.Usage()
		os.Exit(1)
	}

	addr := resolveAdaptorAddr(*adaptorAddr)
	conn, client := dialAdaptor(addr)
	defer conn.Close()

	resp, err := client.StartSession(context.Background(), &adapterpb.StartSessionRequest{
		ZoneId: *zoneID,
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "gRPC error: %v\n", err)
		os.Exit(1)
	}

	printAny(map[string]any{
		"session_id": resp.SessionId,
		"active":     resp.Active,
		"zone_id":    resp.ZoneId,
		"start_time": resp.StartTime,
	})
}

func runStopSession(args []string) {
	fs := flag.NewFlagSet("stop-session", flag.ContinueOnError)
	fs.SetOutput(os.Stderr)
	adaptorAddr := fs.String("adaptor-addr", "", "PARKING_OPERATOR_ADAPTOR address")

	if err := fs.Parse(args); err != nil {
		os.Exit(1)
	}

	addr := resolveAdaptorAddr(*adaptorAddr)
	conn, client := dialAdaptor(addr)
	defer conn.Close()

	resp, err := client.StopSession(context.Background(), &adapterpb.StopSessionRequest{})
	if err != nil {
		fmt.Fprintf(os.Stderr, "gRPC error: %v\n", err)
		os.Exit(1)
	}

	printAny(map[string]any{
		"session_id": resp.SessionId,
		"status":     "stopped",
		"active":     resp.Active,
	})
}
