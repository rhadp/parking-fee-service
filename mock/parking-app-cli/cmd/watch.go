package cmd

import (
	"context"
	"fmt"
	"io"
	"os"
	"os/signal"
	"syscall"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/config"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/grpcclient"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"

	pb "github.com/parking-fee-service/mock/parking-app-cli/internal/gen/update_service/v1"
)

func runWatch(args []string) error {
	addr := config.UpdateServiceAddr()
	conn, err := grpcclient.Dial(addr)
	if err != nil {
		return err
	}
	defer conn.Close()

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Handle Ctrl+C gracefully
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigCh
		cancel()
	}()

	client := pb.NewUpdateServiceClient(conn)
	stream, err := client.WatchAdapterStates(ctx, &pb.WatchAdapterStatesRequest{})
	if err != nil {
		return fmt.Errorf("WatchAdapterStates RPC to %s: %w", addr, err)
	}

	for {
		event, err := stream.Recv()
		if err == io.EOF {
			return nil
		}
		if err != nil {
			// Context cancelled means clean shutdown
			if ctx.Err() != nil {
				return nil
			}
			return fmt.Errorf("receiving event: %w", err)
		}

		if printErr := output.PrintJSON(map[string]interface{}{
			"adapter_id": event.AdapterId,
			"old_state":  event.OldState.String(),
			"new_state":  event.NewState.String(),
			"timestamp":  event.Timestamp,
		}); printErr != nil {
			return printErr
		}
	}
}
