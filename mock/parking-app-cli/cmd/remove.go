package cmd

import (
	"context"
	"time"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/grpcclient"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"
	updatepb "github.com/parking-fee-service/proto/update_service/v1"
)

// RunRemove executes the remove subcommand.
// It calls UPDATE_SERVICE.RemoveAdapter via gRPC.
func RunRemove(args []string, serviceAddr string) error {
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

	resp, err := client.RemoveAdapter(ctx, &updatepb.RemoveAdapterRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		return err
	}

	return output.PrintJSON(map[string]any{
		"success": resp.GetSuccess(),
	})
}
