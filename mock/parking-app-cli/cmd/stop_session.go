package cmd

import (
	"context"
	"time"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/grpcclient"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"
	adaptorpb "github.com/parking-fee-service/proto"
)

// RunStopSession executes the stop-session subcommand.
// It calls PARKING_OPERATOR_ADAPTOR.StopSession via gRPC.
func RunStopSession(args []string, adaptorAddr string) error {
	_, err := requireFlag(args, "session-id")
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

	resp, err := client.StopSession(ctx, &adaptorpb.StopSessionRequest{})
	if err != nil {
		return err
	}

	return output.PrintJSON(map[string]any{
		"session_id":       resp.GetSessionId(),
		"duration_seconds": resp.GetDurationSeconds(),
		"fee":              resp.GetFee(),
		"status":           resp.GetStatus(),
	})
}
