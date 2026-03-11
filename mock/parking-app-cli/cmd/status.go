package cmd

import (
	"context"
	"time"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/grpcclient"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"
	updatepb "github.com/parking-fee-service/proto/update_service/v1"
)

// RunStatus executes the status subcommand.
// It calls UPDATE_SERVICE.GetAdapterStatus via gRPC.
func RunStatus(args []string, serviceAddr string) error {
	adapterID, err := requireFlag(args, "adapter-id")
	if err != nil {
		return err
	}

	conn, err := grpcclient.Dial(serviceAddr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	resp, err := client.GetAdapterStatus(ctx, &updatepb.GetAdapterStatusRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		return err
	}

	return output.PrintJSON(map[string]any{
		"adapter_id":    resp.GetAdapterId(),
		"image_ref":     resp.GetImageRef(),
		"state":         resp.GetState().String(),
		"error_message": resp.GetErrorMessage(),
	})
}
