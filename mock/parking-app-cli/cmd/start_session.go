package cmd

import (
	"context"
	"time"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/grpcclient"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"
	adaptorpb "github.com/parking-fee-service/proto"
)

// RunStartSession executes the start-session subcommand.
// It calls PARKING_OPERATOR_ADAPTOR.StartSession via gRPC.
func RunStartSession(args []string, adaptorAddr string) error {
	zoneID, err := requireFlag(args, "zone-id")
	if err != nil {
		return err
	}

	conn, err := grpcclient.Dial(adaptorAddr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := adaptorpb.NewParkingAdaptorClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.StartSession(ctx, &adaptorpb.StartSessionRequest{
		ZoneId: zoneID,
	})
	if err != nil {
		return err
	}

	return output.PrintJSON(map[string]any{
		"session_id": resp.GetSessionId(),
		"status":     resp.GetStatus(),
	})
}
