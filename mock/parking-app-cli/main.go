package main

import (
	"context"
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
	if len(os.Args) < 2 {
		fmt.Fprintf(os.Stderr, "Usage: %s <subcommand> [flags]\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "Subcommands: lookup, adapter-info, install, list, watch, status, remove, start-session, stop-session\n")
		os.Exit(1)
	}

	subcommand := os.Args[1]

	switch subcommand {
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
		fmt.Fprintf(os.Stderr, "Error: unknown subcommand %q\n", subcommand)
		os.Exit(1)
	}
}

// ---------------------------------------------------------------------------
// REST subcommands
// ---------------------------------------------------------------------------

func runLookup() {
	lat := resolveFlag("--lat", "")
	lon := resolveFlag("--lon", "")
	if lat == "" || lon == "" {
		fmt.Fprintf(os.Stderr, "Error: --lat and --lon are required\n")
		os.Exit(1)
	}

	serviceAddr := resolveFlag("--service-addr", "PARKING_FEE_SERVICE_ADDR")
	if serviceAddr == "" {
		serviceAddr = "http://localhost:8080"
	}

	url := fmt.Sprintf("%s/operators?lat=%s&lon=%s", serviceAddr, lat, lon)
	doHTTPGet(url)
}

func runAdapterInfo() {
	operatorID := resolveFlag("--operator-id", "")
	if operatorID == "" {
		fmt.Fprintf(os.Stderr, "Error: --operator-id is required\n")
		os.Exit(1)
	}

	serviceAddr := resolveFlag("--service-addr", "PARKING_FEE_SERVICE_ADDR")
	if serviceAddr == "" {
		serviceAddr = "http://localhost:8080"
	}

	url := fmt.Sprintf("%s/operators/%s/adapter", serviceAddr, operatorID)
	doHTTPGet(url)
}

// doHTTPGet performs a GET request and prints the response body to stdout.
func doHTTPGet(url string) {
	resp, err := http.Get(url)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: request failed: %v\n", err)
		os.Exit(1)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: failed to read response: %v\n", err)
		os.Exit(1)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		fmt.Fprintf(os.Stderr, "Error: HTTP %d: %s\n", resp.StatusCode, string(body))
		os.Exit(1)
	}

	fmt.Println(string(body))
}

// ---------------------------------------------------------------------------
// gRPC subcommands: UPDATE_SERVICE
// ---------------------------------------------------------------------------

func connectUpdateService() (updatepb.UpdateServiceClient, *grpc.ClientConn) {
	addr := resolveFlag("--update-addr", "UPDATE_SERVICE_ADDR")
	if addr == "" {
		addr = "localhost:50052"
	}

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	conn, err := grpc.DialContext(ctx, addr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: failed to connect to UPDATE_SERVICE at %s: %v\n", addr, err)
		os.Exit(1)
	}

	return updatepb.NewUpdateServiceClient(conn), conn
}

func runInstall() {
	imageRef := resolveFlag("--image-ref", "")
	checksum := resolveFlag("--checksum", "")
	if imageRef == "" || checksum == "" {
		fmt.Fprintf(os.Stderr, "Error: --image-ref and --checksum are required\n")
		os.Exit(1)
	}

	client, conn := connectUpdateService()
	defer conn.Close()

	resp, err := client.InstallAdapter(context.Background(), &updatepb.InstallAdapterRequest{
		ImageRef:       imageRef,
		ChecksumSha256: checksum,
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: InstallAdapter RPC failed: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("job_id: %s\nadapter_id: %s\nstate: %s\n", resp.GetJobId(), resp.GetAdapterId(), resp.GetState().String())
}

func runList() {
	client, conn := connectUpdateService()
	defer conn.Close()

	resp, err := client.ListAdapters(context.Background(), &updatepb.ListAdaptersRequest{})
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: ListAdapters RPC failed: %v\n", err)
		os.Exit(1)
	}

	for _, a := range resp.GetAdapters() {
		fmt.Printf("adapter_id: %s  state: %s  image_ref: %s\n", a.GetAdapterId(), a.GetState().String(), a.GetImageRef())
	}
}

func runWatch() {
	client, conn := connectUpdateService()
	defer conn.Close()

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Handle SIGINT for graceful shutdown.
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigCh
		cancel()
	}()

	stream, err := client.WatchAdapterStates(ctx, &updatepb.WatchAdapterStatesRequest{})
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: WatchAdapterStates RPC failed: %v\n", err)
		os.Exit(1)
	}

	for {
		event, err := stream.Recv()
		if err != nil {
			if err == io.EOF || ctx.Err() != nil {
				return
			}
			fmt.Fprintf(os.Stderr, "Error: stream error: %v\n", err)
			os.Exit(1)
		}
		fmt.Printf("adapter_id: %s  old_state: %s  new_state: %s  timestamp: %d\n",
			event.GetAdapterId(), event.GetOldState().String(), event.GetNewState().String(), event.GetTimestamp())
	}
}

func runAdapterStatus() {
	adapterID := resolveFlag("--adapter-id", "")
	if adapterID == "" {
		fmt.Fprintf(os.Stderr, "Error: --adapter-id is required\n")
		os.Exit(1)
	}

	client, conn := connectUpdateService()
	defer conn.Close()

	resp, err := client.GetAdapterStatus(context.Background(), &updatepb.GetAdapterStatusRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: GetAdapterStatus RPC failed: %v\n", err)
		os.Exit(1)
	}

	a := resp.GetAdapter()
	fmt.Printf("adapter_id: %s\nstate: %s\nimage_ref: %s\n", a.GetAdapterId(), a.GetState().String(), a.GetImageRef())
}

func runRemove() {
	adapterID := resolveFlag("--adapter-id", "")
	if adapterID == "" {
		fmt.Fprintf(os.Stderr, "Error: --adapter-id is required\n")
		os.Exit(1)
	}

	client, conn := connectUpdateService()
	defer conn.Close()

	resp, err := client.RemoveAdapter(context.Background(), &updatepb.RemoveAdapterRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: RemoveAdapter RPC failed: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("adapter_id: %s\nstate: %s\n", resp.GetAdapterId(), resp.GetState().String())
}

// ---------------------------------------------------------------------------
// gRPC subcommands: PARKING_OPERATOR_ADAPTOR
// ---------------------------------------------------------------------------

func connectAdaptorService() (adaptorpb.ParkingOperatorAdaptorServiceClient, *grpc.ClientConn) {
	addr := resolveFlag("--adaptor-addr", "ADAPTOR_ADDR")
	if addr == "" {
		addr = "localhost:50053"
	}

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	conn, err := grpc.DialContext(ctx, addr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: failed to connect to PARKING_OPERATOR_ADAPTOR at %s: %v\n", addr, err)
		os.Exit(1)
	}

	return adaptorpb.NewParkingOperatorAdaptorServiceClient(conn), conn
}

func runStartSession() {
	zoneID := resolveFlag("--zone-id", "")
	if zoneID == "" {
		fmt.Fprintf(os.Stderr, "Error: --zone-id is required\n")
		os.Exit(1)
	}

	client, conn := connectAdaptorService()
	defer conn.Close()

	resp, err := client.StartSession(context.Background(), &adaptorpb.StartSessionRequest{
		ZoneId: zoneID,
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: StartSession RPC failed: %v\n", err)
		os.Exit(1)
	}

	session := resp.GetSession()
	status := "stopped"
	if session.GetActive() {
		status = "active"
	}
	fmt.Printf("session_id: %s\nstatus: %s\nzone_id: %s\n", session.GetSessionId(), status, session.GetZoneId())
}

func runStopSession() {
	client, conn := connectAdaptorService()
	defer conn.Close()

	resp, err := client.StopSession(context.Background(), &adaptorpb.StopSessionRequest{})
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: StopSession RPC failed: %v\n", err)
		os.Exit(1)
	}

	session := resp.GetSession()
	status := "stopped"
	if session.GetActive() {
		status = "active"
	}
	fmt.Printf("session_id: %s\nstatus: %s\nzone_id: %s\n", session.GetSessionId(), status, session.GetZoneId())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// resolveFlag extracts a --flag=value from os.Args, falling back to env var.
func resolveFlag(flag, envVar string) string {
	prefix := flag + "="
	for _, arg := range os.Args[2:] {
		if v, ok := strings.CutPrefix(arg, prefix); ok {
			return v
		}
	}
	if envVar != "" {
		return os.Getenv(envVar)
	}
	return ""
}
