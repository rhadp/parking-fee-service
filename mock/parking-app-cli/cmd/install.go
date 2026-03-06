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

func runInstall(args []string) error {
	fs := flag.NewFlagSet("install", flag.ContinueOnError)
	imageRef := fs.String("image-ref", "", "OCI image reference (required)")
	checksum := fs.String("checksum", "", "SHA256 checksum (required)")

	if err := fs.Parse(args); err != nil {
		return err
	}

	if *imageRef == "" || *checksum == "" {
		return fmt.Errorf("usage: parking-app-cli install --image-ref=<ref> --checksum=<sha256>\n  both --image-ref and --checksum are required")
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

	resp, err := client.InstallAdapter(ctx, &pb.InstallAdapterRequest{
		ImageRef:       *imageRef,
		ChecksumSha256: *checksum,
	})
	if err != nil {
		return fmt.Errorf("InstallAdapter RPC to %s: %w", addr, err)
	}

	return output.PrintJSON(map[string]interface{}{
		"job_id":     resp.JobId,
		"adapter_id": resp.AdapterId,
		"state":      resp.State.String(),
	})
}
