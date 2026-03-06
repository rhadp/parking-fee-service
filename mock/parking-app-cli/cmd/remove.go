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

func runRemove(args []string) error {
	fs := flag.NewFlagSet("remove", flag.ContinueOnError)
	adapterID := fs.String("adapter-id", "", "Adapter ID (required)")

	if err := fs.Parse(args); err != nil {
		return err
	}

	if *adapterID == "" {
		return fmt.Errorf("usage: parking-app-cli remove --adapter-id=<id>\n  --adapter-id is required")
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

	resp, err := client.RemoveAdapter(ctx, &pb.RemoveAdapterRequest{
		AdapterId: *adapterID,
	})
	if err != nil {
		return fmt.Errorf("RemoveAdapter RPC to %s: %w", addr, err)
	}

	return output.PrintJSON(map[string]interface{}{
		"success": resp.Success,
	})
}
