package cmd

import (
	"context"
	"flag"
	"fmt"
	"time"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/config"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/grpcclient"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"

	pb "github.com/parking-fee-service/mock/parking-app-cli/internal/gen/parking_adaptor"
)

func runStopSession(args []string) error {
	fs := flag.NewFlagSet("stop-session", flag.ContinueOnError)
	sessionID := fs.String("session-id", "", "Session ID (required)")

	if err := fs.Parse(args); err != nil {
		return err
	}

	if *sessionID == "" {
		return fmt.Errorf("usage: parking-app-cli stop-session --session-id=<id>\n  --session-id is required")
	}

	addr := config.ParkingAdaptorAddr()
	conn, err := grpcclient.Dial(addr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := pb.NewParkingAdaptorClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.StopSession(ctx, &pb.StopSessionRequest{})
	if err != nil {
		return fmt.Errorf("StopSession RPC to %s: %w", addr, err)
	}

	return output.PrintJSON(map[string]interface{}{
		"session_id":       resp.SessionId,
		"duration_seconds": resp.DurationSeconds,
		"fee":              resp.Fee,
		"status":           resp.Status,
	})
}
