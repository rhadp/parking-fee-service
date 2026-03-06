package cmd

import (
	"context"
	"fmt"
	"time"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/config"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/grpcclient"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"

	pb "github.com/parking-fee-service/mock/parking-app-cli/internal/gen/update_service/v1"
)

func runList(args []string) error {
	addr := config.UpdateServiceAddr()
	conn, err := grpcclient.Dial(addr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := pb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.ListAdapters(ctx, &pb.ListAdaptersRequest{})
	if err != nil {
		return fmt.Errorf("ListAdapters RPC to %s: %w", addr, err)
	}

	adapters := make([]map[string]interface{}, 0, len(resp.Adapters))
	for _, a := range resp.Adapters {
		adapters = append(adapters, map[string]interface{}{
			"adapter_id": a.AdapterId,
			"image_ref":  a.ImageRef,
			"state":      a.State.String(),
		})
	}

	return output.PrintJSON(adapters)
}
