package cmd

import (
	"context"
	"flag"
	"fmt"
	"time"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/config"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/grpcclient"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"

	pb "github.com/parking-fee-service/mock/parking-app-cli/internal/gen/update_service/v1"
)

func runStatus(args []string) error {
	fs := flag.NewFlagSet("status", flag.ContinueOnError)
	adapterID := fs.String("adapter-id", "", "Adapter ID (required)")

	if err := fs.Parse(args); err != nil {
		return err
	}

	if *adapterID == "" {
		return fmt.Errorf("usage: parking-app-cli status --adapter-id=<id>\n  --adapter-id is required")
	}

	addr := config.UpdateServiceAddr()
	conn, err := grpcclient.Dial(addr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := pb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.GetAdapterStatus(ctx, &pb.GetAdapterStatusRequest{
		AdapterId: *adapterID,
	})
	if err != nil {
		return fmt.Errorf("GetAdapterStatus RPC to %s: %w", addr, err)
	}

	return output.PrintJSON(map[string]interface{}{
		"adapter_id":    resp.AdapterId,
		"image_ref":     resp.ImageRef,
		"state":         resp.State.String(),
		"error_message": resp.ErrorMessage,
	})
}
