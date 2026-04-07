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

	adapterpb "parking-fee-service/gen/go/adapter"
	updatepb "parking-fee-service/gen/go/update"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(args []string) error {
	if len(args) == 0 {
		return fmt.Errorf("usage: parking-app-cli <subcommand> [flags]")
	}

	subcommand := args[0]
	flags := parseFlags(args[1:])

	switch subcommand {
	// REST subcommands (PARKING_FEE_SERVICE)
	case "lookup":
		return cmdLookup(flags)
	case "adapter-info":
		return cmdAdapterInfo(flags)

	// gRPC subcommands (UPDATE_SERVICE)
	case "install":
		return cmdInstall(flags)
	case "list":
		return cmdList(flags)
	case "watch":
		return cmdWatch(flags)
	case "status":
		return cmdAdapterStatus(flags)
	case "remove":
		return cmdRemove(flags)

	// Session override subcommands (PARKING_OPERATOR_ADAPTOR)
	case "start-session":
		return cmdStartSession(flags)
	case "stop-session":
		return cmdStopSession(flags)

	default:
		return fmt.Errorf("unknown subcommand: %s", subcommand)
	}
}

// ---------------------------------------------------------------------------
// REST subcommands (PARKING_FEE_SERVICE)
// ---------------------------------------------------------------------------

func resolveServiceAddr(flags map[string]string) string {
	if v := flags["service-addr"]; v != "" {
		return v
	}
	if v := os.Getenv("PARKING_FEE_SERVICE_ADDR"); v != "" {
		return v
	}
	return "http://localhost:8080"
}

func cmdLookup(flags map[string]string) error {
	lat := flags["lat"]
	lon := flags["lon"]
	if lat == "" || lon == "" {
		return fmt.Errorf("error: --lat and --lon are required\nusage: parking-app-cli lookup --lat=<lat> --lon=<lon>")
	}

	addr := resolveServiceAddr(flags)
	url := fmt.Sprintf("%s/operators?lat=%s&lon=%s", addr, lat, lon)

	return doRESTGet(url)
}

func cmdAdapterInfo(flags map[string]string) error {
	operatorID := flags["operator-id"]
	if operatorID == "" {
		return fmt.Errorf("error: --operator-id is required\nusage: parking-app-cli adapter-info --operator-id=<id>")
	}

	addr := resolveServiceAddr(flags)
	url := fmt.Sprintf("%s/operators/%s/adapter", addr, operatorID)

	return doRESTGet(url)
}

func doRESTGet(url string) error {
	resp, err := http.Get(url)
	if err != nil {
		return fmt.Errorf("error: %v", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("error reading response: %v", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("HTTP %d: %s", resp.StatusCode, string(body))
	}

	fmt.Print(string(body))
	return nil
}

// ---------------------------------------------------------------------------
// gRPC subcommands (UPDATE_SERVICE)
// ---------------------------------------------------------------------------

func resolveUpdateAddr(flags map[string]string) string {
	if v := flags["update-addr"]; v != "" {
		return v
	}
	if v := os.Getenv("UPDATE_SERVICE_ADDR"); v != "" {
		return v
	}
	return "localhost:50052"
}

func dialUpdateService(flags map[string]string) (updatepb.UpdateServiceClient, *grpc.ClientConn, error) {
	addr := resolveUpdateAddr(flags)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	conn, err := grpc.DialContext(ctx, addr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		return nil, nil, fmt.Errorf("error connecting to UPDATE_SERVICE at %s: %v", addr, err)
	}
	return updatepb.NewUpdateServiceClient(conn), conn, nil
}

func cmdInstall(flags map[string]string) error {
	imageRef := flags["image-ref"]
	checksum := flags["checksum"]
	if imageRef == "" || checksum == "" {
		return fmt.Errorf("error: --image-ref and --checksum are required\nusage: parking-app-cli install --image-ref=<ref> --checksum=<sha256>")
	}

	client, conn, err := dialUpdateService(flags)
	if err != nil {
		return err
	}
	defer conn.Close()

	resp, err := client.InstallAdapter(context.Background(), &updatepb.InstallAdapterRequest{
		ImageRef:      imageRef,
		ChecksumSha256: checksum,
	})
	if err != nil {
		return fmt.Errorf("gRPC error: %v", err)
	}

	return printJSON(resp)
}

func cmdList(flags map[string]string) error {
	client, conn, err := dialUpdateService(flags)
	if err != nil {
		return err
	}
	defer conn.Close()

	resp, err := client.ListAdapters(context.Background(), &updatepb.ListAdaptersRequest{})
	if err != nil {
		return fmt.Errorf("gRPC error: %v", err)
	}

	return printJSON(resp)
}

func cmdWatch(flags map[string]string) error {
	client, conn, err := dialUpdateService(flags)
	if err != nil {
		return err
	}
	defer conn.Close()

	adapterID := flags["adapter-id"]
	stream, err := client.WatchAdapterStates(context.Background(), &updatepb.WatchAdapterStatesRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		return fmt.Errorf("gRPC error: %v", err)
	}

	// Set up signal handler for graceful stop
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)

	doneCh := make(chan error, 1)
	go func() {
		for {
			event, err := stream.Recv()
			if err == io.EOF {
				doneCh <- nil
				return
			}
			if err != nil {
				doneCh <- fmt.Errorf("stream error: %v", err)
				return
			}
			if err := printJSON(event); err != nil {
				doneCh <- err
				return
			}
			fmt.Println()
		}
	}()

	select {
	case <-sigCh:
		return nil
	case err := <-doneCh:
		return err
	}
}

func cmdAdapterStatus(flags map[string]string) error {
	adapterID := flags["adapter-id"]
	if adapterID == "" {
		return fmt.Errorf("error: --adapter-id is required\nusage: parking-app-cli status --adapter-id=<id>")
	}

	client, conn, err := dialUpdateService(flags)
	if err != nil {
		return err
	}
	defer conn.Close()

	resp, err := client.GetAdapterStatus(context.Background(), &updatepb.GetAdapterStatusRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		return fmt.Errorf("gRPC error: %v", err)
	}

	return printJSON(resp)
}

func cmdRemove(flags map[string]string) error {
	adapterID := flags["adapter-id"]
	if adapterID == "" {
		return fmt.Errorf("error: --adapter-id is required\nusage: parking-app-cli remove --adapter-id=<id>")
	}

	client, conn, err := dialUpdateService(flags)
	if err != nil {
		return err
	}
	defer conn.Close()

	resp, err := client.RemoveAdapter(context.Background(), &updatepb.RemoveAdapterRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		return fmt.Errorf("gRPC error: %v", err)
	}

	return printJSON(resp)
}

// ---------------------------------------------------------------------------
// Session override subcommands (PARKING_OPERATOR_ADAPTOR)
// ---------------------------------------------------------------------------

func resolveAdaptorAddr(flags map[string]string) string {
	if v := flags["adaptor-addr"]; v != "" {
		return v
	}
	if v := os.Getenv("ADAPTOR_ADDR"); v != "" {
		return v
	}
	return "localhost:50053"
}

func dialAdaptorService(flags map[string]string) (adapterpb.AdapterServiceClient, *grpc.ClientConn, error) {
	addr := resolveAdaptorAddr(flags)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	conn, err := grpc.DialContext(ctx, addr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		return nil, nil, fmt.Errorf("error connecting to PARKING_OPERATOR_ADAPTOR at %s: %v", addr, err)
	}
	return adapterpb.NewAdapterServiceClient(conn), conn, nil
}

func cmdStartSession(flags map[string]string) error {
	zoneID := flags["zone-id"]
	if zoneID == "" {
		return fmt.Errorf("error: --zone-id is required\nusage: parking-app-cli start-session --zone-id=<zone>")
	}

	client, conn, err := dialAdaptorService(flags)
	if err != nil {
		return err
	}
	defer conn.Close()

	resp, err := client.StartSession(context.Background(), &adapterpb.StartSessionRequest{
		ZoneId: zoneID,
	})
	if err != nil {
		return fmt.Errorf("gRPC error: %v", err)
	}

	return printJSON(resp)
}

func cmdStopSession(flags map[string]string) error {
	client, conn, err := dialAdaptorService(flags)
	if err != nil {
		return err
	}
	defer conn.Close()

	resp, err := client.StopSession(context.Background(), &adapterpb.StopSessionRequest{})
	if err != nil {
		return fmt.Errorf("gRPC error: %v", err)
	}

	return printJSON(resp)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

func printJSON(v any) error {
	data, err := json.MarshalIndent(v, "", "  ")
	if err != nil {
		return fmt.Errorf("error encoding JSON: %v", err)
	}
	fmt.Println(string(data))
	return nil
}

// parseFlags parses --key=value flags into a map.
func parseFlags(args []string) map[string]string {
	flags := make(map[string]string)
	for _, arg := range args {
		if strings.HasPrefix(arg, "--") {
			kv := strings.TrimPrefix(arg, "--")
			parts := strings.SplitN(kv, "=", 2)
			if len(parts) == 2 {
				flags[parts[0]] = parts[1]
			}
		}
	}
	return flags
}
