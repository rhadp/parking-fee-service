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

func runStartSession(args []string) error {
	fs := flag.NewFlagSet("start-session", flag.ContinueOnError)
	zoneID := fs.String("zone-id", "", "Zone ID (required)")

	if err := fs.Parse(args); err != nil {
		return err
	}

	if *zoneID == "" {
		return fmt.Errorf("usage: parking-app-cli start-session --zone-id=<id>\n  --zone-id is required")
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

	resp, err := client.StartSession(ctx, &pb.StartSessionRequest{
		ZoneId: *zoneID,
	})
	if err != nil {
		return fmt.Errorf("StartSession RPC to %s: %w", addr, err)
	}

	return output.PrintJSON(map[string]interface{}{
		"session_id": resp.SessionId,
		"status":     resp.Status,
	})
}
