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

	adaptorpb "github.com/rhadp/parking-fee-service/gen/adapter"
	updatepb "github.com/rhadp/parking-fee-service/gen/update"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}

// run parses the subcommand and flags, then dispatches to the appropriate handler.
func run(args []string) error {
	if len(args) == 0 {
		return fmt.Errorf("usage: parking-app-cli <subcommand> [flags]")
	}

	subcmd := args[0]
	flags := parseFlags(args[1:])

	switch subcmd {
	// REST subcommands (PARKING_FEE_SERVICE)
	case "lookup":
		return doLookup(flags)
	case "adapter-info":
		return doAdapterInfo(flags)

	// gRPC subcommands (UPDATE_SERVICE)
	case "install":
		return doInstall(flags)
	case "list":
		return doList(flags)
	case "watch":
		return doWatch(flags)
	case "status":
		return doAdapterStatus(flags)
	case "remove":
		return doRemove(flags)

	// gRPC subcommands (PARKING_OPERATOR_ADAPTOR)
	case "start-session":
		return doStartSession(flags)
	case "stop-session":
		return doStopSession(flags)

	default:
		return fmt.Errorf("unknown subcommand: %s", subcmd)
	}
}

// ---------------------------------------------------------------------------
// REST subcommands — PARKING_FEE_SERVICE
// ---------------------------------------------------------------------------

// doLookup sends GET /operators?lat=...&lon=... to PARKING_FEE_SERVICE.
func doLookup(flags map[string]string) error {
	lat := flags["lat"]
	lon := flags["lon"]
	if lat == "" || lon == "" {
		return fmt.Errorf("missing required flags: --lat and --lon")
	}

	addr := flagOrEnvDefault(flags, "service-addr", "PARKING_FEE_SERVICE_ADDR", "http://localhost:8080")
	url := fmt.Sprintf("%s/operators?lat=%s&lon=%s", addr, lat, lon)
	return doHTTPGetPrint(url)
}

// doAdapterInfo sends GET /operators/{id}/adapter to PARKING_FEE_SERVICE.
func doAdapterInfo(flags map[string]string) error {
	operatorID := flags["operator-id"]
	if operatorID == "" {
		return fmt.Errorf("missing required flag: --operator-id")
	}

	addr := flagOrEnvDefault(flags, "service-addr", "PARKING_FEE_SERVICE_ADDR", "http://localhost:8080")
	url := fmt.Sprintf("%s/operators/%s/adapter", addr, operatorID)
	return doHTTPGetPrint(url)
}

// doHTTPGetPrint performs an HTTP GET and prints the response body to stdout.
func doHTTPGetPrint(url string) error {
	resp, err := http.Get(url)
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed to read response: %w", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("HTTP %d: %s", resp.StatusCode, string(body))
	}

	fmt.Println(string(body))
	return nil
}

// ---------------------------------------------------------------------------
// gRPC subcommands — UPDATE_SERVICE
// ---------------------------------------------------------------------------

// connectUpdate creates a gRPC connection to UPDATE_SERVICE.
func connectUpdate(flags map[string]string) (updatepb.UpdateServiceClient, *grpc.ClientConn, error) {
	addr := flagOrEnvDefault(flags, "update-addr", "UPDATE_SERVICE_ADDR", "localhost:50052")
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return nil, nil, fmt.Errorf("gRPC connect: %w", err)
	}
	return updatepb.NewUpdateServiceClient(conn), conn, nil
}

// doInstall calls InstallAdapter on UPDATE_SERVICE.
func doInstall(flags map[string]string) error {
	imageRef := flags["image-ref"]
	checksum := flags["checksum"]
	if imageRef == "" || checksum == "" {
		return fmt.Errorf("missing required flags: --image-ref and --checksum")
	}

	client, conn, err := connectUpdate(flags)
	if err != nil {
		return err
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	resp, err := client.InstallAdapter(ctx, &updatepb.InstallAdapterRequest{
		ImageRef:       imageRef,
		ChecksumSha256: checksum,
	})
	if err != nil {
		return fmt.Errorf("InstallAdapter RPC failed: %w", err)
	}

	printJSON(map[string]any{
		"job_id":     resp.GetJobId(),
		"adapter_id": resp.GetAdapterId(),
		"state":      resp.GetState().String(),
	})
	return nil
}

// doList calls ListAdapters on UPDATE_SERVICE.
func doList(flags map[string]string) error {
	client, conn, err := connectUpdate(flags)
	if err != nil {
		return err
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	resp, err := client.ListAdapters(ctx, &updatepb.ListAdaptersRequest{})
	if err != nil {
		return fmt.Errorf("ListAdapters RPC failed: %w", err)
	}

	adapters := make([]map[string]any, 0, len(resp.GetAdapters()))
	for _, a := range resp.GetAdapters() {
		adapters = append(adapters, map[string]any{
			"adapter_id": a.GetAdapterId(),
			"image_ref":  a.GetImageRef(),
			"state":      a.GetState().String(),
		})
	}
	printJSON(map[string]any{"adapters": adapters})
	return nil
}

// doWatch calls WatchAdapterStates on UPDATE_SERVICE and streams events.
func doWatch(flags map[string]string) error {
	client, conn, err := connectUpdate(flags)
	if err != nil {
		return err
	}
	defer conn.Close()

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
		return fmt.Errorf("WatchAdapterStates RPC failed: %w", err)
	}

	for {
		event, err := stream.Recv()
		if err != nil {
			if err == io.EOF || ctx.Err() != nil {
				return nil
			}
			return fmt.Errorf("stream error: %w", err)
		}
		printJSON(map[string]any{
			"adapter_id": event.GetAdapterId(),
			"old_state":  event.GetOldState().String(),
			"new_state":  event.GetNewState().String(),
			"timestamp":  event.GetTimestamp(),
		})
	}
}

// doAdapterStatus calls GetAdapterStatus on UPDATE_SERVICE.
func doAdapterStatus(flags map[string]string) error {
	adapterID := flags["adapter-id"]
	if adapterID == "" {
		return fmt.Errorf("missing required flag: --adapter-id")
	}

	client, conn, err := connectUpdate(flags)
	if err != nil {
		return err
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	resp, err := client.GetAdapterStatus(ctx, &updatepb.GetAdapterStatusRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		return fmt.Errorf("GetAdapterStatus RPC failed: %w", err)
	}

	a := resp.GetAdapter()
	if a != nil {
		printJSON(map[string]any{
			"adapter_id": a.GetAdapterId(),
			"image_ref":  a.GetImageRef(),
			"state":      a.GetState().String(),
		})
	} else {
		printJSON(map[string]any{"adapter_id": adapterID})
	}
	return nil
}

// doRemove calls RemoveAdapter on UPDATE_SERVICE.
func doRemove(flags map[string]string) error {
	adapterID := flags["adapter-id"]
	if adapterID == "" {
		return fmt.Errorf("missing required flag: --adapter-id")
	}

	client, conn, err := connectUpdate(flags)
	if err != nil {
		return err
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	resp, err := client.RemoveAdapter(ctx, &updatepb.RemoveAdapterRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		return fmt.Errorf("RemoveAdapter RPC failed: %w", err)
	}

	printJSON(map[string]any{
		"adapter_id": resp.GetAdapterId(),
		"state":      resp.GetState().String(),
	})
	return nil
}

// ---------------------------------------------------------------------------
// gRPC subcommands — PARKING_OPERATOR_ADAPTOR
// ---------------------------------------------------------------------------

// connectAdaptor creates a gRPC connection to PARKING_OPERATOR_ADAPTOR.
func connectAdaptor(flags map[string]string) (adaptorpb.ParkingOperatorAdaptorServiceClient, *grpc.ClientConn, error) {
	addr := flagOrEnvDefault(flags, "adaptor-addr", "ADAPTOR_ADDR", "localhost:50053")
	conn, err := grpc.NewClient(addr, grpc.WithTransportCredentials(insecure.NewCredentials()))
	if err != nil {
		return nil, nil, fmt.Errorf("gRPC connect: %w", err)
	}
	return adaptorpb.NewParkingOperatorAdaptorServiceClient(conn), conn, nil
}

// doStartSession calls StartSession on PARKING_OPERATOR_ADAPTOR.
func doStartSession(flags map[string]string) error {
	zoneID := flags["zone-id"]
	if zoneID == "" {
		return fmt.Errorf("missing required flag: --zone-id")
	}

	client, conn, err := connectAdaptor(flags)
	if err != nil {
		return err
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	resp, err := client.StartSession(ctx, &adaptorpb.StartSessionRequest{
		ZoneId: zoneID,
	})
	if err != nil {
		return fmt.Errorf("StartSession RPC failed: %w", err)
	}

	session := resp.GetSession()
	if session != nil {
		printJSON(map[string]any{
			"session_id": session.GetSessionId(),
			"active":     session.GetActive(),
			"zone_id":    session.GetZoneId(),
			"start_time": session.GetStartTime(),
		})
	} else {
		printJSON(map[string]any{"status": "started"})
	}
	return nil
}

// doStopSession calls StopSession on PARKING_OPERATOR_ADAPTOR.
func doStopSession(flags map[string]string) error {
	client, conn, err := connectAdaptor(flags)
	if err != nil {
		return err
	}
	defer conn.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	resp, err := client.StopSession(ctx, &adaptorpb.StopSessionRequest{})
	if err != nil {
		return fmt.Errorf("StopSession RPC failed: %w", err)
	}

	session := resp.GetSession()
	if session != nil {
		printJSON(map[string]any{
			"session_id": session.GetSessionId(),
			"active":     session.GetActive(),
			"zone_id":    session.GetZoneId(),
		})
	} else {
		printJSON(map[string]any{"status": "stopped"})
	}
	return nil
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

// printJSON marshals a value to JSON and prints it to stdout.
func printJSON(v any) {
	b, _ := json.MarshalIndent(v, "", "  ")
	fmt.Println(string(b))
}

// parseFlags extracts --key=value pairs from args.
func parseFlags(args []string) map[string]string {
	flags := make(map[string]string)
	for _, arg := range args {
		if val, ok := strings.CutPrefix(arg, "--"); ok {
			parts := strings.SplitN(val, "=", 2)
			if len(parts) == 2 {
				flags[parts[0]] = parts[1]
			}
		}
	}
	return flags
}

// flagOrEnv returns the flag value if present, otherwise the env var value.
func flagOrEnv(flags map[string]string, flagName, envName string) string {
	if v, ok := flags[flagName]; ok {
		return v
	}
	return os.Getenv(envName)
}

// flagOrEnvDefault returns the flag value, then env var, then a default value.
func flagOrEnvDefault(flags map[string]string, flagName, envName, def string) string {
	if v := flagOrEnv(flags, flagName, envName); v != "" {
		return v
	}
	return def
}
