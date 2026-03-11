package cmd

import (
	"context"
	"time"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/grpcclient"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"
	updatepb "github.com/parking-fee-service/proto/update_service/v1"
)

// RunList executes the list subcommand.
// It calls UPDATE_SERVICE.ListAdapters via gRPC.
func RunList(args []string, serviceAddr string) error {
	conn, err := grpcclient.Dial(serviceAddr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.ListAdapters(ctx, &updatepb.ListAdaptersRequest{})
	if err != nil {
		return err
	}

	adapters := make([]map[string]any, 0, len(resp.GetAdapters()))
	for _, a := range resp.GetAdapters() {
		adapters = append(adapters, map[string]any{
			"adapter_id": a.GetAdapterId(),
			"image_ref":  a.GetImageRef(),
			"state":      a.GetState().String(),
		})
	}

	return output.PrintJSON(adapters)
}
