// parking-app-cli simulates the PARKING_APP on AAOS IVI.
// It queries PARKING_FEE_SERVICE (REST), triggers adapter install via UPDATE_SERVICE (gRPC),
// and overrides adapter sessions via PARKING_OPERATOR_ADAPTOR (gRPC).
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"time"

	adaptoripb "github.com/sdv-demo/parking-fee-service/mock/parking-app-cli/pb/adaptor"
	updatepb "github.com/sdv-demo/parking-fee-service/mock/parking-app-cli/pb/update"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

const (
	defaultServiceAddr = "http://localhost:8080"
	defaultUpdateAddr  = "localhost:50052"
	defaultAdaptorAddr = "localhost:50053"
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(args []string) error {
	if len(args) == 0 {
		fmt.Println("parking-app-cli v0.1.0")
		return nil
	}

	subcommand := args[0]
	rest := args[1:]
	switch subcommand {
	case "lookup":
		return lookupCmd(rest)
	case "adapter-info":
		return adapterInfoCmd(rest)
	case "install":
		return installCmd(rest)
	case "list":
		return listCmd(rest)
	case "watch":
		return watchCmd(rest)
	case "status":
		return adapterStatusCmd(rest)
	case "remove":
		return removeCmd(rest)
	case "start-session":
		return startSessionCmd(rest)
	case "stop-session":
		return stopSessionCmd(rest)
	default:
		return fmt.Errorf("unknown subcommand %q\nUsage: parking-app-cli <subcommand> [flags]", subcommand)
	}
}

// ── Flag parsing ──────────────────────────────────────────────────────────────

// parseFlags parses simple --key=value or --key value style flags.
func parseFlags(args []string, known map[string]*string) error {
	i := 0
	for i < len(args) {
		arg := args[i]
		if len(arg) < 2 || arg[0] != '-' {
			return fmt.Errorf("unexpected argument: %q", arg)
		}
		key := arg
		for len(key) > 0 && key[0] == '-' {
			key = key[1:]
		}
		val := ""
		hasEq := false
		for j, c := range key {
			if c == '=' {
				val = key[j+1:]
				key = key[:j]
				hasEq = true
				break
			}
		}
		if ptr, ok := known[key]; ok {
			if hasEq {
				*ptr = val
			} else if i+1 < len(args) {
				i++
				*ptr = args[i]
			} else {
				return fmt.Errorf("flag --%s requires a value", key)
			}
		} else {
			return fmt.Errorf("unknown flag: --%s", key)
		}
		i++
	}
	return nil
}

func resolveServiceAddr(flagVal string) string {
	if flagVal != "" {
		return flagVal
	}
	if v := os.Getenv("PARKING_FEE_SERVICE_ADDR"); v != "" {
		return v
	}
	return defaultServiceAddr
}

func resolveUpdateAddr(flagVal string) string {
	if flagVal != "" {
		return flagVal
	}
	if v := os.Getenv("UPDATE_SERVICE_ADDR"); v != "" {
		return v
	}
	return defaultUpdateAddr
}

func resolveAdaptorAddr(flagVal string) string {
	if flagVal != "" {
		return flagVal
	}
	if v := os.Getenv("ADAPTOR_ADDR"); v != "" {
		return v
	}
	return defaultAdaptorAddr
}

// ── REST helpers ──────────────────────────────────────────────────────────────

func doGet(url string) error {
	resp, err := http.Get(url) //nolint:noctx
	if err != nil {
		return fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()
	body, _ := io.ReadAll(resp.Body)
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("HTTP %d: %s", resp.StatusCode, string(body))
	}
	fmt.Print(string(body))
	return nil
}

// ── gRPC helpers ──────────────────────────────────────────────────────────────

func dialUpdate(addr string) (*grpc.ClientConn, error) {
	conn, err := grpc.NewClient(addr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to UPDATE_SERVICE at %s: %w", addr, err)
	}
	return conn, nil
}

func dialAdaptor(addr string) (*grpc.ClientConn, error) {
	conn, err := grpc.NewClient(addr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to PARKING_OPERATOR_ADAPTOR at %s: %w", addr, err)
	}
	return conn, nil
}

func printJSON(v any) {
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	_ = enc.Encode(v)
}

// ── REST subcommands ──────────────────────────────────────────────────────────

func lookupCmd(args []string) error {
	var lat, lon, serviceAddr string
	flags := map[string]*string{
		"lat":          &lat,
		"lon":          &lon,
		"service-addr": &serviceAddr,
	}
	if err := parseFlags(args, flags); err != nil {
		return fmt.Errorf("%w\nUsage: parking-app-cli lookup --lat=<lat> --lon=<lon> [--service-addr=<addr>]", err)
	}
	if lat == "" || lon == "" {
		return fmt.Errorf("--lat and --lon are required\nUsage: parking-app-cli lookup --lat=<lat> --lon=<lon> [--service-addr=<addr>]")
	}
	addr := resolveServiceAddr(serviceAddr)
	url := fmt.Sprintf("%s/operators?lat=%s&lon=%s", addr, lat, lon)
	return doGet(url)
}

func adapterInfoCmd(args []string) error {
	var operatorID, serviceAddr string
	flags := map[string]*string{
		"operator-id":  &operatorID,
		"service-addr": &serviceAddr,
	}
	if err := parseFlags(args, flags); err != nil {
		return fmt.Errorf("%w\nUsage: parking-app-cli adapter-info --operator-id=<id> [--service-addr=<addr>]", err)
	}
	if operatorID == "" {
		return fmt.Errorf("--operator-id is required\nUsage: parking-app-cli adapter-info --operator-id=<id>")
	}
	addr := resolveServiceAddr(serviceAddr)
	url := fmt.Sprintf("%s/operators/%s/adapter", addr, operatorID)
	return doGet(url)
}

// ── gRPC UPDATE_SERVICE subcommands ─────────────────────────────────────────

func installCmd(args []string) error {
	var imageRef, checksum, updateAddr string
	flags := map[string]*string{
		"image-ref":   &imageRef,
		"checksum":    &checksum,
		"update-addr": &updateAddr,
	}
	if err := parseFlags(args, flags); err != nil {
		return fmt.Errorf("%w\nUsage: parking-app-cli install --image-ref=<ref> --checksum=<sha256> [--update-addr=<addr>]", err)
	}
	if imageRef == "" || checksum == "" {
		return fmt.Errorf("--image-ref and --checksum are required\nUsage: parking-app-cli install --image-ref=<ref> --checksum=<sha256>")
	}
	addr := resolveUpdateAddr(updateAddr)
	conn, err := dialUpdate(addr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	resp, err := client.InstallAdapter(ctx, &updatepb.InstallAdapterRequest{
		ImageRef:       imageRef,
		ChecksumSha256: checksum,
	})
	if err != nil {
		return fmt.Errorf("InstallAdapter RPC failed: %w", err)
	}
	printJSON(map[string]any{
		"job_id":     resp.JobId,
		"adapter_id": resp.AdapterId,
		"state":      resp.State.String(),
	})
	return nil
}

func listCmd(args []string) error {
	var updateAddr string
	flags := map[string]*string{
		"update-addr": &updateAddr,
	}
	if err := parseFlags(args, flags); err != nil {
		return fmt.Errorf("%w\nUsage: parking-app-cli list [--update-addr=<addr>]", err)
	}
	addr := resolveUpdateAddr(updateAddr)
	conn, err := dialUpdate(addr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	resp, err := client.ListAdapters(ctx, &updatepb.ListAdaptersRequest{})
	if err != nil {
		return fmt.Errorf("ListAdapters RPC failed: %w", err)
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
	return nil
}

func watchCmd(args []string) error {
	var updateAddr string
	flags := map[string]*string{
		"update-addr": &updateAddr,
	}
	if err := parseFlags(args, flags); err != nil {
		return fmt.Errorf("%w\nUsage: parking-app-cli watch [--update-addr=<addr>]", err)
	}
	addr := resolveUpdateAddr(updateAddr)
	conn, err := dialUpdate(addr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 24*time.Hour)
	defer cancel()

	stream, err := client.WatchAdapterStates(ctx, &updatepb.WatchAdapterStatesRequest{})
	if err != nil {
		return fmt.Errorf("WatchAdapterStates RPC failed: %w", err)
	}
	for {
		event, err := stream.Recv()
		if err != nil {
			if err == io.EOF {
				return nil
			}
			return fmt.Errorf("stream error: %w", err)
		}
		printJSON(map[string]any{
			"adapter_id": event.AdapterId,
			"old_state":  event.OldState.String(),
			"new_state":  event.NewState.String(),
			"timestamp":  event.Timestamp,
		})
	}
}

func adapterStatusCmd(args []string) error {
	var adapterID, updateAddr string
	flags := map[string]*string{
		"adapter-id":  &adapterID,
		"update-addr": &updateAddr,
	}
	if err := parseFlags(args, flags); err != nil {
		return fmt.Errorf("%w\nUsage: parking-app-cli status --adapter-id=<id> [--update-addr=<addr>]", err)
	}
	if adapterID == "" {
		return fmt.Errorf("--adapter-id is required\nUsage: parking-app-cli status --adapter-id=<id>")
	}
	addr := resolveUpdateAddr(updateAddr)
	conn, err := dialUpdate(addr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	resp, err := client.GetAdapterStatus(ctx, &updatepb.GetAdapterStatusRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		return fmt.Errorf("GetAdapterStatus RPC failed: %w", err)
	}
	if resp.Adapter != nil {
		printJSON(map[string]any{
			"adapter_id": resp.Adapter.AdapterId,
			"image_ref":  resp.Adapter.ImageRef,
			"state":      resp.Adapter.State.String(),
		})
	}
	return nil
}

func removeCmd(args []string) error {
	var adapterID, updateAddr string
	flags := map[string]*string{
		"adapter-id":  &adapterID,
		"update-addr": &updateAddr,
	}
	if err := parseFlags(args, flags); err != nil {
		return fmt.Errorf("%w\nUsage: parking-app-cli remove --adapter-id=<id> [--update-addr=<addr>]", err)
	}
	if adapterID == "" {
		return fmt.Errorf("--adapter-id is required\nUsage: parking-app-cli remove --adapter-id=<id>")
	}
	addr := resolveUpdateAddr(updateAddr)
	conn, err := dialUpdate(addr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	_, err = client.RemoveAdapter(ctx, &updatepb.RemoveAdapterRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		return fmt.Errorf("RemoveAdapter RPC failed: %w", err)
	}
	fmt.Println("adapter removed")
	return nil
}

// ── gRPC PARKING_OPERATOR_ADAPTOR subcommands ────────────────────────────────

func startSessionCmd(args []string) error {
	var zoneID, adaptorAddr string
	flags := map[string]*string{
		"zone-id":      &zoneID,
		"adaptor-addr": &adaptorAddr,
	}
	if err := parseFlags(args, flags); err != nil {
		return fmt.Errorf("%w\nUsage: parking-app-cli start-session --zone-id=<zone> [--adaptor-addr=<addr>]", err)
	}
	if zoneID == "" {
		return fmt.Errorf("--zone-id is required\nUsage: parking-app-cli start-session --zone-id=<zone>")
	}
	addr := resolveAdaptorAddr(adaptorAddr)
	conn, err := dialAdaptor(addr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := adaptoripb.NewParkingAdaptorClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	resp, err := client.StartSession(ctx, &adaptoripb.StartSessionRequest{
		ZoneId: zoneID,
	})
	if err != nil {
		return fmt.Errorf("StartSession RPC failed: %w", err)
	}
	printJSON(map[string]any{
		"session_id": resp.SessionId,
		"status":     resp.Status,
	})
	return nil
}

func stopSessionCmd(args []string) error {
	var adaptorAddr string
	flags := map[string]*string{
		"adaptor-addr": &adaptorAddr,
	}
	if err := parseFlags(args, flags); err != nil {
		return fmt.Errorf("%w\nUsage: parking-app-cli stop-session [--adaptor-addr=<addr>]", err)
	}
	addr := resolveAdaptorAddr(adaptorAddr)
	conn, err := dialAdaptor(addr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := adaptoripb.NewParkingAdaptorClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	resp, err := client.StopSession(ctx, &adaptoripb.StopSessionRequest{})
	if err != nil {
		return fmt.Errorf("StopSession RPC failed: %w", err)
	}
	printJSON(map[string]any{
		"session_id":       resp.SessionId,
		"status":           resp.Status,
		"duration_seconds": resp.DurationSeconds,
		"total_amount":     resp.TotalAmount,
		"currency":         resp.Currency,
	})
	return nil
}
