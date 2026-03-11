package cmd

import (
	"context"
	"time"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/grpcclient"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"
	updatepb "github.com/parking-fee-service/proto/update_service/v1"
)

// RunInstall executes the install subcommand.
// It calls UPDATE_SERVICE.InstallAdapter via gRPC.
func RunInstall(args []string, serviceAddr string) error {
	imageRef, err := requireFlag(args, "image-ref")
	if err != nil {
		return err
	}
	checksum, err := requireFlag(args, "checksum")
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

	resp, err := client.InstallAdapter(ctx, &updatepb.InstallAdapterRequest{
		ImageRef:      imageRef,
		ChecksumSha256: checksum,
	})
	if err != nil {
		return err
	}

	return output.PrintJSON(map[string]any{
		"job_id":     resp.GetJobId(),
		"adapter_id": resp.GetAdapterId(),
		"state":      resp.GetState().String(),
	})
}
